//! The 3D model element in the editor (spec 100, D-130): a background renderer that draws
//! models into textures without blocking the interface, the encapsulated 3D editing session
//! on the canvas (rotate, move, scale and camera gizmos, the orientation cube, the tool bar,
//! Apply and Cancel), and the «Objeto 3D» properties.
//!
//! The canvas never draws 3D every frame: it shows the last picture of each model and asks
//! for a new one only when the model, its size or its quality changes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{Align2, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use gafira_doc::{
    ContentHash, Model3dLighting, Model3dNode, Model3dProjection, NodeId, PageId, Placement,
};
use gafira_io_model3d::{
    ImportError, ImportLimits, ImportWarning, ModelDecoder, ModelFormat, camera_of, detect_format,
    params_of, pose_of, render_parallel,
};
use gafira_model3d::{Pose, ViewPreset};
use gafira_ports::export::ImageDecoder;
use gafira_ports::media::MediaError;
use gafira_raster::ImageBuf;

use crate::canvas::{HANDLES, Handle};
use crate::charts::plain;
use crate::icons::{self, Icon, Weight};
use crate::slider::{self, Slider};
use crate::style::Theme;
use crate::text::Texts;

/// The file extensions of 3D models (spec 100 §3.1).
pub(crate) const EXTENSIONS: [&str; 2] = ["glb", "obj"];

/// Whether `path` names a 3D model, by its extension.
pub(crate) fn is_model_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// The catalog key of an import error (spec 100 §9: the cause and what to do).
pub(crate) fn error_key(error: &ImportError) -> &'static str {
    match error {
        ImportError::TooLarge { .. }
        | ImportError::Scene(
            gafira_model3d::SceneError::TooManyVertices(_)
            | gafira_model3d::SceneError::TooManyTriangles(_)
            | gafira_model3d::SceneError::TooManyMaterials(_)
            | gafira_model3d::SceneError::TooManyTextures(_)
            | gafira_model3d::SceneError::TooManyPrimitives(_)
            | gafira_model3d::SceneError::TextureTooLarge { .. },
        ) => "model3d.error_large",
        ImportError::UnknownFormat => "model3d.error_format",
        ImportError::UnsupportedExtension(_) => "model3d.error_extension",
        ImportError::ExternalReference => "model3d.error_external",
        ImportError::Damaged(_) | ImportError::Scene(_) | ImportError::Cancelled => {
            "model3d.error_damaged"
        }
    }
}

/// The catalog key of an import warning.
pub(crate) fn warning_key(warning: &ImportWarning) -> Option<&'static str> {
    match warning {
        ImportWarning::MissingMaterials => Some("model3d.warn_materials"),
        ImportWarning::NonTriangleParts | ImportWarning::TextureSkipped => {
            Some("model3d.warn_parts")
        }
        ImportWarning::IgnoredExtension(_) => None,
    }
}

/// Reads and checks a model file before it joins the document: its format, or why not.
pub(crate) fn check_file(
    bytes: &[u8],
    name: &str,
) -> Result<(ModelFormat, Vec<ImportWarning>), ImportError> {
    let format = detect_format(bytes, name).ok_or(ImportError::UnknownFormat)?;
    let imported = gafira_io_model3d::import(bytes, format, &ImportLimits::default(), &|| false)?;
    Ok((format, imported.warnings))
}

// ---------------------------------------------------------------------------------------
// The background renderer.

/// What a picture is for: a model on the canvas, a quick view tile, or a file of Biblioteca.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Slot {
    Node(NodeId),
    Preset(u8),
    File(PathBuf),
}

#[derive(Clone)]
enum Source {
    Asset(ContentHash, Arc<Vec<u8>>),
    File(PathBuf),
}

struct Job {
    slot: Slot,
    key: u64,
    source: Source,
    model: Model3dNode,
    size: (u32, u32),
    samples: u32,
}

struct Done {
    slot: Slot,
    key: u64,
    image: Option<egui::ColorImage>,
}

/// An `ImageDecoder` without images: the renderer only draws models.
struct NoImages;

impl ImageDecoder for NoImages {
    fn decode(&self, _bytes: &[u8]) -> Result<ImageBuf, MediaError> {
        Err(MediaError::Corrupt {
            detail: "no images here".to_owned(),
        })
    }
}

/// Draws models on one worker thread, the newest request of each slot first; the editor
/// keeps showing the last picture of a slot until the new one arrives.
#[derive(Default)]
pub(crate) struct Studio {
    jobs: Option<Sender<Job>>,
    done: Option<Receiver<Done>>,
    textures: BTreeMap<u64, egui::TextureHandle>,
    latest: BTreeMap<Slot, u64>,
    asked: BTreeSet<u64>,
    failed: BTreeSet<u64>,
    bytes: BTreeMap<ContentHash, Arc<Vec<u8>>>,
}

fn key_of(model: &Model3dNode, size: (u32, u32), samples: u32, extra: u64) -> u64 {
    // FNV-1a over the picture key, the size and the quality.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let shown = model.shown();
    let words = [size.0 as u64, size.1 as u64, samples as u64, extra];
    for byte in shown
        .as_hash()
        .0
        .iter()
        .copied()
        .chain(words.iter().flat_map(|w| w.to_le_bytes()))
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn worker(ctx: egui::Context, jobs: Receiver<Job>, done: Sender<Done>) {
    let decoder = ModelDecoder::new(NoImages);
    let mut files: Vec<(PathBuf, Arc<gafira_model3d::Scene>)> = Vec::new();
    while let Ok(first) = jobs.recv() {
        let mut batch = vec![first];
        while let Ok(job) = jobs.try_recv() {
            batch.push(job);
        }
        // Only the newest request of each slot counts; models on the canvas first.
        let mut newest: BTreeMap<Slot, Job> = BTreeMap::new();
        for job in batch {
            newest.insert(job.slot.clone(), job);
        }
        for (_, job) in newest {
            let scene = match &job.source {
                Source::Asset(hash, bytes) => decoder
                    .scene(*hash, bytes, gafira_io_model3d::format_of(job.model.format))
                    .ok(),
                Source::File(path) => match files.iter().find(|(p, _)| p == path) {
                    Some((_, scene)) => Some(Arc::clone(scene)),
                    None => {
                        let read = std::fs::read(path).ok().and_then(|bytes| {
                            let name = path.to_string_lossy();
                            let format = detect_format(&bytes, &name)?;
                            gafira_io_model3d::import(
                                &bytes,
                                format,
                                &ImportLimits::default(),
                                &|| false,
                            )
                            .ok()
                        });
                        read.map(|imported| {
                            let scene = Arc::new(imported.scene);
                            files.push((path.clone(), Arc::clone(&scene)));
                            if files.len() > 8 {
                                files.remove(0);
                            }
                            scene
                        })
                    }
                },
            };
            let image = scene.map(|scene| {
                let params = params_of(&job.model, job.size, job.samples).clamped();
                let rgba = render_parallel(&scene, &params);
                egui::ColorImage::from_rgba_unmultiplied(
                    [params.width as usize, params.height as usize],
                    &rgba,
                )
            });
            if done
                .send(Done {
                    slot: job.slot,
                    key: job.key,
                    image,
                })
                .is_err()
            {
                return;
            }
            ctx.request_repaint();
        }
    }
}

impl Studio {
    fn send(&mut self, ctx: &egui::Context, job: Job) {
        if self.jobs.is_none() {
            let (jobs_tx, jobs_rx) = channel();
            let (done_tx, done_rx) = channel();
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new()
                .name("gafira-3d".to_owned())
                .spawn(move || worker(ctx, jobs_rx, done_tx));
            if spawned.is_err() {
                return;
            }
            self.jobs = Some(jobs_tx);
            self.done = Some(done_rx);
        }
        self.asked.insert(job.key);
        if let Some(jobs) = &self.jobs
            && jobs.send(job).is_err()
        {
            self.jobs = None;
            self.done = None;
        }
    }

    /// Takes the finished pictures; call once a frame.
    pub(crate) fn pump(&mut self, ctx: &egui::Context) {
        let Some(done) = &self.done else { return };
        let arrived: Vec<Done> = done.try_iter().collect();
        for d in arrived {
            self.asked.remove(&d.key);
            match d.image {
                Some(image) => {
                    let texture = ctx.load_texture(
                        format!("model3d-{:x}", d.key),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.textures.insert(d.key, texture);
                    self.latest.insert(d.slot, d.key);
                }
                None => {
                    self.failed.insert(d.key);
                }
            }
        }
        // Keep the pictures still shown, and a few more.
        if self.textures.len() > 96 {
            let live: BTreeSet<u64> = self.latest.values().copied().collect();
            self.textures.retain(|k, _| live.contains(k));
        }
    }

    fn bytes_of(
        &mut self,
        read: &dyn Fn(ContentHash) -> Option<Vec<u8>>,
        hash: ContentHash,
    ) -> Option<Arc<Vec<u8>>> {
        if let Some(b) = self.bytes.get(&hash) {
            return Some(Arc::clone(b));
        }
        let bytes = Arc::new(read(hash)?);
        if self.bytes.len() > 16 {
            self.bytes.clear();
        }
        self.bytes.insert(hash, Arc::clone(&bytes));
        Some(bytes)
    }

    /// The picture of `model` for `slot` at `size` pixels: the exact one when drawn, else the
    /// last one of the slot while the new one is drawn. `read` reads a model file by hash.
    pub(crate) fn picture(
        &mut self,
        ctx: &egui::Context,
        read: &dyn Fn(ContentHash) -> Option<Vec<u8>>,
        slot: Slot,
        model: &Model3dNode,
        size: (u32, u32),
        samples: u32,
    ) -> Option<egui::TextureId> {
        let key = key_of(model, size, samples, 0);
        if let Some(texture) = self.textures.get(&key) {
            self.latest.insert(slot, key);
            return Some(texture.id());
        }
        if !self.asked.contains(&key) && !self.failed.contains(&key) {
            let hash = model.asset.as_hash();
            if let Some(bytes) = self.bytes_of(read, hash) {
                let job = Job {
                    slot: slot.clone(),
                    key,
                    source: Source::Asset(hash, bytes),
                    model: model.clone(),
                    size,
                    samples,
                };
                self.send(ctx, job);
            }
        }
        self.latest
            .get(&slot)
            .and_then(|k| self.textures.get(k))
            .map(egui::TextureHandle::id)
    }

    /// A preview of the model file at `path` (Biblioteca, spec 100): the initial view.
    pub(crate) fn file_picture(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        side: u32,
    ) -> Option<egui::TextureId> {
        let mut extra: u64 = 0xf11e;
        for b in path.as_os_str().as_encoded_bytes() {
            extra = (extra ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        let model = Model3dNode::new(
            gafira_doc::timeline::AssetRef::new(ContentHash([0; 32])),
            gafira_doc::Model3dFormat::Glb,
        );
        let key = key_of(&model, (side, side), 2, extra);
        if let Some(texture) = self.textures.get(&key) {
            return Some(texture.id());
        }
        if !self.asked.contains(&key) && !self.failed.contains(&key) {
            let job = Job {
                slot: Slot::File(path.to_path_buf()),
                key,
                source: Source::File(path.to_path_buf()),
                model,
                size: (side, side),
                samples: 2,
            };
            self.send(ctx, job);
        }
        None
    }
}

// ---------------------------------------------------------------------------------------
// The editing session.

/// The tools of the 3D editor (spec 100 §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tool {
    Rotate,
    Move,
    Scale,
    Camera,
}

impl Tool {
    const ALL: [Tool; 4] = [Tool::Rotate, Tool::Move, Tool::Scale, Tool::Camera];

    fn icon(self) -> Icon {
        match self {
            Self::Rotate => Icon::RotateRight,
            Self::Move => Icon::MoveAxes,
            Self::Scale => Icon::ScaleOut,
            Self::Camera => Icon::Camera,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Rotate => "model3d.rotate",
            Self::Move => "model3d.move",
            Self::Scale => "model3d.scale",
            Self::Camera => "model3d.camera",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Rotate => "model3d.hint_rotate",
            Self::Move => "model3d.hint_move",
            Self::Scale => "model3d.hint_scale",
            Self::Camera => "model3d.hint_camera",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Grip {
    /// Rotate freely like a trackball.
    Free,
    /// The blue ring: only sideways.
    Yaw,
    /// The green ring: only up and down.
    Pitch,
    /// Move along one axis of the model frame, whose screen direction per unit is given.
    Axis(usize, Vec2),
    /// Move in the plane of the screen.
    Plane,
    /// A handle of the element's frame: its box changes size, the model with it.
    Frame(Handle),
    /// Nothing under the pointer to take (inside the frame with Escalar).
    Nothing,
    Orbit,
}

#[derive(Clone, Debug)]
struct Drag {
    start: Pos2,
    grip: Grip,
    before: Model3dNode,
    /// The element's box when the drag started.
    box_before: Placement,
}

/// An open 3D editing session of one element (spec 100 §3.3, §3.5): its changes stay here
/// until Apply makes them one undo step; Cancel drops them.
#[derive(Clone, Debug)]
pub(crate) struct Session {
    pub(crate) page: PageId,
    pub(crate) node: NodeId,
    pub(crate) original: Model3dNode,
    pub(crate) current: Model3dNode,
    /// The element's box on the page when the session opened, and now: Escalar changes
    /// the size the model takes on the page (spec 100, `canvas_scale`).
    pub(crate) original_placement: Placement,
    pub(crate) placement: Placement,
    pub(crate) tool: Tool,
    undo: Vec<(Model3dNode, Placement)>,
    redo: Vec<(Model3dNode, Placement)>,
    drag: Option<Drag>,
    /// Where the element was on the screen in the last frame, for the wheel.
    pub(crate) rect: Option<Rect>,
    /// The last change was the wheel: more wheel joins the same step.
    merging: bool,
}

impl Session {
    pub(crate) fn new(
        page: PageId,
        node: NodeId,
        model: Model3dNode,
        placement: Placement,
    ) -> Self {
        Self {
            page,
            node,
            original: model.clone(),
            current: model,
            original_placement: placement,
            placement,
            tool: Tool::Rotate,
            undo: Vec::new(),
            redo: Vec::new(),
            drag: None,
            rect: None,
            merging: false,
        }
    }

    pub(crate) fn modified(&self) -> bool {
        self.current != self.original || self.placement != self.original_placement
    }

    /// Whether the element's box changed in this session.
    pub(crate) fn resized(&self) -> bool {
        self.placement != self.original_placement
    }

    pub(crate) fn push_undo(&mut self) {
        self.undo.push((self.current.clone(), self.placement));
        self.redo.clear();
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
    }

    pub(crate) fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Sets the model, keeping the step for the session's undo.
    pub(crate) fn set(&mut self, model: Model3dNode) {
        self.merging = false;
        if model != self.current && model.is_valid() {
            self.push_undo();
            self.current = model;
        }
    }

    /// Sets the model as part of the step under way (a slider being dragged, the wheel):
    /// the first change of a run opens the step, the rest join it.
    pub(crate) fn adjust(&mut self, model: Model3dNode) {
        if !model.is_valid() {
            return;
        }
        if self.merging {
            self.current = model;
        } else {
            self.set(model);
            self.merging = true;
        }
    }

    pub(crate) fn undo(&mut self) {
        if let Some((model, placement)) = self.undo.pop() {
            self.redo.push((self.current.clone(), self.placement));
            self.current = model;
            self.placement = placement;
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some((model, placement)) = self.redo.pop() {
            self.undo.push((self.current.clone(), self.placement));
            self.current = model;
            self.placement = placement;
        }
    }

    /// The mouse wheel over the element zooms the camera (spec 100 §3.4), never the page.
    pub(crate) fn wheel(&mut self, ctx: &egui::Context) {
        let Some(rect) = self.rect else { return };
        let over = ctx
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| rect.contains(p));
        if !over {
            return;
        }
        let scroll = ctx.input_mut(|i| {
            let s = i.smooth_scroll_delta.y;
            i.smooth_scroll_delta = Vec2::ZERO;
            i.raw_scroll_delta = Vec2::ZERO;
            let z = i.zoom_delta();
            // A pinch or Ctrl + wheel zooms the camera too.
            let pinch = if (z - 1.0).abs() > f32::EPSILON {
                z.ln() * 400.0
            } else {
                0.0
            };
            for event in &mut i.events {
                if let egui::Event::Zoom(f) = event {
                    *f = 1.0;
                }
            }
            s + pinch
        });
        if scroll.abs() > 0.01 {
            let mut next = self.current.clone();
            next.camera.zoom = (next.camera.zoom * f64::from((scroll * 0.002).exp())).clamp(
                gafira_doc::Model3dCamera::MIN_ZOOM,
                gafira_doc::Model3dCamera::MAX_ZOOM,
            );
            // A run of wheel turns is one step of the session's undo.
            self.adjust(next);
        }
    }
}

/// What the overlay asks of the editor.
#[derive(Default)]
pub(crate) struct Asks {
    pub(crate) apply: bool,
    pub(crate) cancel: bool,
}

fn to_doc_pose(model: &mut Model3dNode, pose: Pose) {
    let pose = pose.sanitized();
    model.rotation = pose.rotation.map(f64::from);
    model.position = pose.position.map(f64::from);
    model.scale = pose.scale.map(f64::from);
}

/// The quick view a camera is at, if any.
pub(crate) fn preset_of(model: &Model3dNode) -> Option<ViewPreset> {
    let c = camera_of(model);
    ViewPreset::matching(c.yaw_degrees, c.elevation_degrees)
}

fn preset_key(preset: Option<ViewPreset>) -> &'static str {
    match preset {
        Some(ViewPreset::Front) => "model3d.front",
        Some(ViewPreset::Back) => "model3d.back",
        Some(ViewPreset::Left) => "model3d.left",
        Some(ViewPreset::Right) => "model3d.right",
        Some(ViewPreset::Top) => "model3d.top",
        Some(ViewPreset::Bottom) => "model3d.bottom",
        Some(ViewPreset::Isometric) => "model3d.iso",
        None => "model3d.custom",
    }
}

/// The model at a quick view.
pub(crate) fn with_preset(model: &Model3dNode, preset: ViewPreset) -> Model3dNode {
    let (yaw, elevation) = preset.angles();
    let mut next = model.clone();
    next.camera.yaw = f64::from(yaw);
    next.camera.elevation = f64::from(elevation);
    next
}

/// The model framed again: centred, at its own size, filling its box (spec 100 «Encuadrar»).
pub(crate) fn fitted(model: &Model3dNode) -> Model3dNode {
    let mut next = model.clone();
    next.position = [0.0; 3];
    next.scale = [1.0; 3];
    next.camera.zoom = 1.0;
    next
}

fn rgb(theme: &Theme, token: &str) -> Color32 {
    theme.c(token)
}

fn project(model: &Model3dNode, rect: Rect, point: [f32; 3]) -> Option<Pos2> {
    let cam = camera_of(model);
    let p = cam.project(point, rect.width(), rect.height())?;
    Some(rect.min + Vec2::new(p[0], p[1]))
}

fn ellipse_points(centre: Pos2, rx: f32, ry: f32, from: f32, to: f32) -> Vec<Pos2> {
    let steps = 72;
    (0..=steps)
        .map(|i| {
            let a = from + (to - from) * i as f32 / steps as f32;
            centre + Vec2::new(rx * a.cos(), ry * a.sin())
        })
        .collect()
}

fn near_ellipse(p: Pos2, centre: Pos2, rx: f32, ry: f32, tolerance: f32) -> bool {
    let d = p - centre;
    let r = ((d.x / rx).powi(2) + (d.y / ry).powi(2)).sqrt();
    let scale = rx.min(ry).max(1.0);
    (r - 1.0).abs() * scale < tolerance
}

fn mix(c: Color32, to: Color32, t: f32) -> Color32 {
    let l = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(c.r(), to.r()), l(c.g(), to.g()), l(c.b(), to.b()), c.a())
}

fn lighter(c: Color32, t: f32) -> Color32 {
    mix(c, Color32::WHITE, t)
}

fn darker(c: Color32, t: f32) -> Color32 {
    mix(c, Color32::BLACK, t)
}

/// The side of a stroke the light comes from (top left): a unit normal of `dir`.
fn lit_side(dir: Vec2) -> Vec2 {
    let n = Vec2::new(-dir.y, dir.x);
    if n.dot(Vec2::new(-1.0, -1.0)) >= 0.0 {
        n
    } else {
        -n
    }
}

/// The layers of a cylinder seen from the side, outside in: width share, shade (negative
/// darker, positive lighter) and the shift toward the light, so the stroke reads as a round
/// tube with no hard line inside it.
const CYLINDER: [(f32, f32, f32); 6] = [
    (1.0, -0.2, 0.0),
    (0.86, -0.08, 0.02),
    (0.7, 0.0, 0.05),
    (0.52, 0.12, 0.1),
    (0.34, 0.26, 0.15),
    (0.16, 0.45, 0.2),
];

fn shade(c: Color32, k: f32) -> Color32 {
    if k < 0.0 {
        darker(c, -k)
    } else {
        lighter(c, k)
    }
}

/// A cylinder along `points`, lit from the top left.
fn tube(painter: &egui::Painter, points: &[Pos2], color: Color32, width: f32) {
    if points.len() < 2 {
        return;
    }
    let dir = (points[points.len() - 1] - points[0]).normalized();
    let lit = if dir.is_finite() {
        lit_side(dir)
    } else {
        Vec2::new(0.0, -1.0)
    };
    for (share, k, shift) in CYLINDER {
        let pts: Vec<Pos2> = points.iter().map(|p| *p + lit * width * shift).collect();
        painter.add(egui::Shape::line(
            pts,
            Stroke::new(width * share, shade(color, k)),
        ));
    }
}

/// A curved cylinder: like [`tube`], with the light toward the top left of each piece.
fn curved_tube(painter: &egui::Painter, points: &[Pos2], color: Color32, width: f32) {
    if points.len() < 2 {
        return;
    }
    let lits: Vec<Vec2> = (0..points.len())
        .map(|i| {
            let (a, b) = if i + 1 < points.len() {
                (points[i], points[i + 1])
            } else {
                (points[i - 1], points[i])
            };
            let d = (b - a).normalized();
            if d.is_finite() {
                lit_side(d)
            } else {
                Vec2::ZERO
            }
        })
        .collect();
    for (share, k, shift) in CYLINDER {
        let pts: Vec<Pos2> = points
            .iter()
            .zip(&lits)
            .map(|(p, l)| *p + *l * width * shift)
            .collect();
        painter.add(egui::Shape::line(
            pts,
            Stroke::new(width * share, shade(color, k)),
        ));
    }
}

/// A cone from its base to its tip: a lit and a shaded face meeting along its axis, so the
/// head of an arrow has volume.
fn cone(painter: &egui::Painter, base: Pos2, tip: Pos2, half: f32, color: Color32) {
    let dir = (tip - base).normalized();
    if !dir.is_finite() {
        return;
    }
    let lit = lit_side(dir);
    let (a, b) = (base + lit * half, base - lit * half);
    painter.add(egui::Shape::convex_polygon(
        vec![tip, a, base],
        lighter(color, 0.2),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![tip, base, b],
        darker(color, 0.15),
        Stroke::NONE,
    ));
}

/// An arrow with volume: a tube and a cone at `to`.
fn arrow(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32, width: f32) {
    let d = to - from;
    let dir = d.normalized();
    if !dir.is_finite() {
        return;
    }
    let head = (width * 3.4).min(d.length() * 0.6);
    let base = to - dir * head;
    tube(painter, &[from, base + dir * 1.0], color, width);
    cone(painter, base, to, width * 1.6, color);
}

/// An arrow head with volume at the end of a curve, pointing along its last piece.
fn curve_head(painter: &egui::Painter, points: &[Pos2], color: Color32, width: f32) {
    let n = points.len();
    if n < 2 {
        return;
    }
    let end = points[n - 1];
    let dir = (end - points[n - 2]).normalized();
    if !dir.is_finite() {
        return;
    }
    // The cone sits on the end of the tube, covering it, and points along it.
    let head = width * 3.0;
    cone(
        painter,
        end - dir * head * 0.35,
        end + dir * head * 0.65,
        width * 1.5,
        color,
    );
}

/// A shaded white ball: the pivot of the gizmos.
fn ball(painter: &egui::Painter, centre: Pos2, r: f32) {
    painter.circle_filled(
        centre + Vec2::new(0.8, 1.6),
        r + 0.5,
        Color32::from_black_alpha(45),
    );
    let steps = 7;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let rr = r * (1.0 - t * 0.85);
        let c = centre + Vec2::new(-0.32 * r, -0.36 * r) * t;
        painter.circle_filled(
            c,
            rr,
            mix(Color32::from_gray(178), Color32::WHITE, (0.25 + t).min(1.0)),
        );
    }
}

/// The points of an ellipse arc split by depth: the far half (`sin < 0`, top of a lying
/// ring) and the near half.
fn split_arc(
    centre: Pos2,
    rx: f32,
    ry: f32,
    from: f32,
    to: f32,
) -> (Vec<Vec<Pos2>>, Vec<Vec<Pos2>>) {
    let steps = 120;
    let mut far = Vec::new();
    let mut near = Vec::new();
    let mut run: Vec<Pos2> = Vec::new();
    let mut run_far = None;
    for i in 0..=steps {
        let a = from + (to - from) * i as f32 / steps as f32;
        let p = centre + Vec2::new(rx * a.cos(), ry * a.sin());
        let is_far = a.sin() < 0.0;
        if run_far.is_some_and(|f| f != is_far) {
            let last = run.last().copied();
            let done = std::mem::take(&mut run);
            if run_far == Some(true) {
                far.push(done)
            } else {
                near.push(done)
            }
            run.extend(last);
        }
        run_far = Some(is_far);
        run.push(p);
    }
    if run_far == Some(true) {
        far.push(run)
    } else {
        near.push(run)
    }
    (far, near)
}

fn badge(painter: &egui::Painter, theme: &Theme, at: Pos2, text: &str, fill: Color32) {
    let galley = painter.layout_no_wrap(text.to_owned(), theme.font("ui-label"), Color32::WHITE);
    let rect = Rect::from_center_size(at, galley.size() + Vec2::new(16.0, 8.0));
    painter.rect_filled(rect, 8.0, fill);
    painter.galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
}

fn degrees_text(texts: &Texts, v: f64) -> String {
    format!("{v:.0}{}", slider::degrees(texts))
}

/// The box of `origin` after dragging the frame handle `handle` by `offset` screen px at
/// `zoom` (spec 100 «Escalar»): a corner keeps the proportions unless `free`; an edge keeps
/// them too, the opposite edge staying and the box centred across, or with `free` moves
/// only its own side.
pub(crate) fn framed(
    origin: &Placement,
    handle: Handle,
    offset: Vec2,
    zoom: f32,
    free: bool,
) -> Placement {
    if crate::resize::is_corner(handle) || free || !crate::rotation::upright(origin) {
        return crate::resize::resized(origin, handle, offset, zoom, !free);
    }
    let (dx, dy) = (f64::from(offset.x / zoom), f64::from(offset.y / zoom));
    let k = if handle.x != 0 {
        (origin.w + f64::from(handle.x) * dx) / origin.w
    } else {
        (origin.h + f64::from(handle.y) * dy) / origin.h
    };
    let least = (crate::canvas::MIN_SIDE / origin.w).max(crate::canvas::MIN_SIDE / origin.h);
    let k = k.max(least);
    if !k.is_finite() {
        return *origin;
    }
    let (w, h) = (origin.w * k, origin.h * k);
    let mut to = *origin;
    to.w = w;
    to.h = h;
    if handle.x != 0 {
        to.x = if handle.x == 1 {
            origin.x
        } else {
            origin.x + origin.w - w
        };
        to.y = origin.y + (origin.h - h) / 2.0;
    } else {
        to.y = if handle.y == 1 {
            origin.y
        } else {
            origin.y + origin.h - h
        };
        to.x = origin.x + (origin.w - w) / 2.0;
    }
    to
}

fn handle_pos(rect: Rect, h: Handle) -> Pos2 {
    Pos2::new(
        rect.center().x + f32::from(h.x) * rect.width() / 2.0,
        rect.center().y + f32::from(h.y) * rect.height() / 2.0,
    )
}

/// The gizmos, the orientation cube, the hint and the tool bar of the session, over the
/// element at `rect` (screen; the session's box at `zoom`) on a canvas whose visible area
/// is `viewport`.
pub(crate) fn overlay(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    session: &mut Session,
    (rect, zoom): (Rect, f32),
    viewport: Rect,
) -> Asks {
    let mut asks = Asks::default();
    session.rect = Some(rect);
    // Under the tool bar and the cube, over the canvas.
    let layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new("model3d.gizmos"));
    let painter = ctx.layer_painter(layer).with_clip_rect(viewport);
    let accent = rgb(theme, "primary.bg");
    let (gx, gy, gz) = (
        rgb(theme, "gizmo.x"),
        rgb(theme, "gizmo.y"),
        rgb(theme, "gizmo.z"),
    );

    // The surface that takes the gestures, a little larger than the element so its frame
    // handles are in reach: nothing behind it moves (spec 100 §3.4).
    let reach = rect.expand(14.0);
    let area = egui::Area::new(egui::Id::new("model3d.surface"))
        .order(egui::Order::Middle)
        .fixed_pos(reach.min)
        .show(ctx, |ui| {
            ui.allocate_exact_size(reach.size(), egui::Sense::click_and_drag())
                .1
        });
    let response = area.inner;
    let pointer = ctx.input(|i| i.pointer.interact_pos());
    let free = ctx.input(|i| i.modifiers.shift);

    let model = session.current.clone();
    let centre = project(&model, rect, model.position.map(|v| v as f32)).unwrap_or(rect.center());
    let radius = rect.width().min(rect.height()) * 0.42;
    let (ring_rx, ring_ry) = (radius, radius * 0.3);
    // The blue ring runs from just past its arrow head round to it; the green one ends at
    // its top, heading right (spec 100 reference).
    use std::f32::consts::{PI, TAU};
    // The blue ring runs behind the model and along its sides; the green one is an arc on
    // the left, from the bottom up to its head at the top (spec 100 reference).
    // Open in front, where it would cross the model, so it reads as going round it; its
    // head ends the arc on the near right.
    let (blue_from, blue_to) = (PI / 2.0 + 0.85, TAU + PI / 2.0 - 0.85);
    let blue_head = (blue_to - 0.12, blue_to);
    let (green_from, green_to) = (PI / 2.0 - 0.3, 1.5 * PI + 0.3);

    // What a press grabs, by tool and place.
    let tool = session.tool;
    let grip_at = |p: Pos2| -> Grip {
        match tool {
            Tool::Rotate => {
                if near_ellipse(p, centre, ring_rx, ring_ry, 12.0) {
                    Grip::Yaw
                } else if near_ellipse(p, centre, ring_ry, ring_rx, 12.0) {
                    Grip::Pitch
                } else {
                    Grip::Free
                }
            }
            Tool::Move => {
                let pose = pose_of(&model);
                let mut best: Option<(f32, Grip)> = None;
                for axis in 0..3 {
                    let mut unit = [0.0_f32; 3];
                    unit[axis] = 0.5;
                    let tip = project(&model, rect, [0, 1, 2].map(|k| pose.position[k] + unit[k]));
                    if let Some(tip) = tip {
                        let d = tip - centre;
                        let len = d.length();
                        if len < 4.0 {
                            continue;
                        }
                        let ends = segment_distance(
                            p,
                            centre,
                            centre + d.normalized() * axis_length(len, radius),
                        );
                        if ends < 12.0 && best.is_none_or(|(b, _)| ends < b) {
                            best = Some((ends, Grip::Axis(axis, d * 2.0)));
                        }
                    }
                }
                best.map_or(Grip::Plane, |(_, g)| g)
            }
            Tool::Scale => HANDLES
                .into_iter()
                .map(|h| (h, (handle_pos(rect, h) - p).length()))
                .filter(|(_, d)| *d < 14.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map_or(Grip::Nothing, |(h, _)| Grip::Frame(h)),
            Tool::Camera => Grip::Orbit,
        }
    };

    if response.drag_started()
        && let Some(p) = pointer
    {
        session.drag = Some(Drag {
            start: p,
            grip: grip_at(p),
            before: session.current.clone(),
            box_before: session.placement,
        });
    }
    if response.dragged()
        && let (Some(drag), Some(p)) = (session.drag.clone(), pointer)
    {
        let d = p - drag.start;
        let mut next = drag.before.clone();
        let cam = camera_of(&drag.before);
        let pose = pose_of(&drag.before);
        let unit = 2.0 / (rect.width().min(rect.height()) * cam.clamped().zoom).max(1.0);
        match drag.grip {
            Grip::Free => to_doc_pose(&mut next, pose.turned_in_view(&cam, d.x * 0.5, d.y * 0.5)),
            Grip::Yaw => to_doc_pose(&mut next, pose.turned_in_view(&cam, d.x * 0.5, 0.0)),
            Grip::Pitch => to_doc_pose(&mut next, pose.turned_in_view(&cam, 0.0, d.y * 0.5)),
            Grip::Plane => {
                let b = cam.basis();
                let shift = b.right * (d.x * unit) - b.up * (d.y * unit);
                let mut moved = pose;
                moved.position = [0, 1, 2].map(|k| pose.position[k] + shift.to_array()[k]);
                to_doc_pose(&mut next, moved);
            }
            Grip::Axis(axis, per_unit) => {
                let len2 = per_unit.length_sq().max(1.0);
                let t = d.dot(per_unit) / len2;
                let mut moved = pose;
                moved.position[axis] += t;
                to_doc_pose(&mut next, moved);
            }
            Grip::Frame(handle) => {
                // The box on the page grows or shrinks; the model fills it at the new size.
                session.placement = framed(&drag.box_before, handle, d, zoom, free);
            }
            Grip::Nothing => {}
            Grip::Orbit => {
                next.camera.yaw = f64::from(gafira_model3d::normalize_degrees(
                    (drag.before.camera.yaw + f64::from(d.x) * 0.4) as f32,
                ));
                next.camera.elevation =
                    (drag.before.camera.elevation + f64::from(d.y) * 0.4).clamp(-90.0, 90.0);
            }
        }
        if next.is_valid() {
            session.current = next;
        }
    }
    if response.drag_stopped()
        && let Some(drag) = session.drag.take()
        && (drag.before != session.current || drag.box_before != session.placement)
    {
        // The whole drag is one step of the session's undo.
        let (model, placement) = (session.current.clone(), session.placement);
        session.current = drag.before;
        session.placement = drag.box_before;
        session.push_undo();
        session.current = model;
        session.placement = placement;
    }

    let hover = pointer.filter(|p| reach.contains(*p));
    let hovered_grip = if session.drag.is_none() {
        hover.map(grip_at)
    } else {
        session.drag.as_ref().map(|d| d.grip)
    };
    if response.hovered() {
        let icon = match hovered_grip {
            Some(Grip::Frame(h)) => {
                if h.x != 0 && h.y != 0 {
                    if h.x == h.y {
                        egui::CursorIcon::ResizeNwSe
                    } else {
                        egui::CursorIcon::ResizeNeSw
                    }
                } else if h.x != 0 {
                    egui::CursorIcon::ResizeHorizontal
                } else {
                    egui::CursorIcon::ResizeVertical
                }
            }
            Some(Grip::Nothing) => egui::CursorIcon::Default,
            _ if session.drag.is_some() => egui::CursorIcon::Grabbing,
            _ => egui::CursorIcon::Grab,
        };
        ctx.set_cursor_icon(icon);
    }

    if session.tool != Tool::Scale {
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.5, accent),
            egui::StrokeKind::Outside,
        );
    }

    // The gizmo of the tool.
    let model = session.current.clone();
    match session.tool {
        Tool::Rotate => {
            let yaw_w = if matches!(hovered_grip, Some(Grip::Yaw)) {
                9.0
            } else {
                7.0
            };
            let pitch_w = if matches!(hovered_grip, Some(Grip::Pitch)) {
                9.0
            } else {
                7.0
            };
            // A glass sphere the rings turn on.
            painter.circle_filled(centre, ring_rx, Color32::from_white_alpha(26));
            painter.circle_stroke(
                centre,
                ring_rx,
                Stroke::new(3.0, Color32::from_white_alpha(170)),
            );
            painter.circle_stroke(
                centre,
                ring_rx - 2.0,
                Stroke::new(1.0, Color32::from_black_alpha(18)),
            );
            let glow = |pts: &[Pos2], c: Color32, w: f32| {
                painter.add(egui::Shape::line(
                    pts.to_vec(),
                    Stroke::new(w * 2.6, c.gamma_multiply(0.12)),
                ));
                painter.add(egui::Shape::line(
                    pts.to_vec(),
                    Stroke::new(w * 1.7, c.gamma_multiply(0.18)),
                ));
            };
            let (blue_far, blue_near) = split_arc(centre, ring_rx, ring_ry, blue_from, blue_to);
            let green = ellipse_points(centre, ring_ry, ring_rx, green_from, green_to);
            // Far half of the blue ring, then the green ring, then the near half in front.
            for run in &blue_far {
                glow(run, gz, yaw_w);
                curved_tube(&painter, run, mix(gz, Color32::WHITE, 0.25), yaw_w);
            }
            glow(&green, gy, pitch_w);
            curved_tube(&painter, &green, gy, pitch_w);
            curve_head(&painter, &green, gy, pitch_w);
            for run in &blue_near {
                glow(run, gz, yaw_w);
                curved_tube(&painter, run, gz, yaw_w);
            }
            let head = ellipse_points(centre, ring_rx, ring_ry, blue_head.0, blue_head.1);
            curve_head(&painter, &head, gz, yaw_w);
            ball(&painter, centre, 9.0);
            if session.drag.is_some() {
                let [x, y, z] = gafira_model3d::euler_degrees_from_quat(pose_of(&model).rotation);
                let text = format!(
                    "{} {} · {} {} · {} {}",
                    plain(texts, "model3d.axis_x"),
                    degrees_text(texts, f64::from(x)),
                    plain(texts, "model3d.axis_y"),
                    degrees_text(texts, f64::from(y)),
                    plain(texts, "model3d.axis_z"),
                    degrees_text(texts, f64::from(z)),
                );
                badge(
                    &painter,
                    theme,
                    Pos2::new(rect.center().x, rect.min.y - 22.0),
                    &text,
                    accent,
                );
            }
        }
        Tool::Move => {
            let pose = pose_of(&model);
            let mut axes: Vec<(usize, Vec2, f32)> = Vec::new();
            let cam = camera_of(&model);
            for axis in 0..3 {
                let mut unit = [0.0_f32; 3];
                unit[axis] = 0.5;
                if let Some(tip) =
                    project(&model, rect, [0, 1, 2].map(|k| pose.position[k] + unit[k]))
                {
                    let depth = cam.basis().back.to_array()[axis];
                    axes.push((axis, tip - centre, depth));
                }
            }
            // The axes going away are drawn first, under the others.
            axes.sort_by(|a, b| a.2.total_cmp(&b.2));
            let r = 11.0;
            for (axis, d, depth) in axes {
                if d.length() < 4.0 {
                    continue;
                }
                let on = matches!(hovered_grip, Some(Grip::Axis(a, _)) if a == axis);
                let color = [gx, gy, gz][axis];
                // An axis pointing away is a little paler, as it is farther.
                let color = if depth < -0.2 {
                    lighter(color, 0.07)
                } else {
                    color
                };
                let dir = d.normalized();
                let tip = centre + dir * axis_length(d.length(), radius);
                arrow(
                    &painter,
                    centre + dir * (r * 0.6),
                    tip,
                    color,
                    if on { 9.0 } else { 7.0 },
                );
            }
            ball(&painter, centre, r);
        }
        Tool::Scale => {
            // The element's frame with its eight handles: dragging one resizes the box on
            // the page, the model filling it at the new size.
            if let Some(drag) = &session.drag
                && drag.box_before != session.placement
            {
                let s = zoom;
                let before = Rect::from_min_size(
                    rect.min
                        - Vec2::new(
                            ((session.placement.x - drag.box_before.x) as f32) * s,
                            ((session.placement.y - drag.box_before.y) as f32) * s,
                        ),
                    Vec2::new(drag.box_before.w as f32 * s, drag.box_before.h as f32 * s),
                );
                painter.add(egui::Shape::dashed_line(
                    &[
                        before.left_top(),
                        before.right_top(),
                        before.right_bottom(),
                        before.left_bottom(),
                        before.left_top(),
                    ],
                    Stroke::new(1.0, accent),
                    6.0,
                    4.0,
                ));
            }
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(2.0, accent),
                egui::StrokeKind::Outside,
            );
            for h in HANDLES {
                let at = handle_pos(rect, h);
                let on = matches!(hovered_grip, Some(Grip::Frame(g)) if g == h);
                let side = if on { 13.0 } else { 10.0 };
                if h.x != 0 && h.y != 0 {
                    let dir = Vec2::new(f32::from(h.x), f32::from(h.y)).normalized();
                    arrow(
                        &painter,
                        at + dir * 10.0,
                        at + dir * 34.0,
                        accent,
                        if on { 7.0 } else { 5.5 },
                    );
                }
                painter.circle_filled(at, side / 2.0 + 1.0, Color32::WHITE);
                painter.circle_stroke(at, side / 2.0 + 1.0, Stroke::new(2.0, accent));
            }
            if session.drag.is_some() {
                let pct = session.placement.w / session.original_placement.w.max(1e-6) * 100.0;
                let text = format!(
                    "{pct:.0}{} · {:.0} × {:.0} px",
                    slider::percent(texts),
                    session.placement.w,
                    session.placement.h
                );
                badge(
                    &painter,
                    theme,
                    Pos2::new(rect.center().x, rect.min.y - 22.0),
                    &text,
                    accent,
                );
            }
        }
        Tool::Camera => {
            let c = rect.center();
            let (rx, ry) = (rect.width() * 0.5, rect.height() * 0.13);
            let base = Pos2::new(c.x, c.y + rect.height() * 0.22);
            let orbit = ellipse_points(base, rx, ry, 0.25, PI - 0.25);
            let orbit: Vec<Pos2> = orbit.into_iter().rev().collect();
            curved_tube(&painter, &orbit, gz, 7.0);
            curve_head(&painter, &orbit, gz, 7.0);
            let arc_c = Pos2::new(c.x, c.y);
            let arc_r = rect.height().min(rect.width()) * 0.48;
            let tilt = ellipse_points(arc_c, arc_r, arc_r, -1.35, -0.3);
            curved_tube(&painter, &tilt, accent, 7.0);
            curve_head(&painter, &tilt, accent, 7.0);
            let turn = format!(
                "{} {}",
                plain(texts, "model3d.turn_short"),
                degrees_text(texts, model.camera.yaw)
            );
            let up = format!(
                "{} {}",
                plain(texts, "model3d.elevation_short"),
                degrees_text(texts, model.camera.elevation)
            );
            badge(&painter, theme, base + Vec2::new(0.0, ry + 4.0), &turn, gz);
            badge(
                &painter,
                theme,
                arc_c + Vec2::new(arc_r * 0.55, -arc_r * 0.95),
                &up,
                accent,
            );
        }
    }

    // The hint: what the tool does, while the pointer is over the element.
    if hover.is_some() && session.drag.is_none() {
        let width = 240.0;
        let right = rect.right() + 16.0 + width < viewport.right();
        let at = if right {
            Pos2::new(rect.right() + 16.0, rect.center().y)
        } else {
            Pos2::new(rect.left() - 16.0 - width, rect.center().y)
        };
        egui::Area::new(egui::Id::new("model3d.hint"))
            .order(egui::Order::Tooltip)
            .fixed_pos(at)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(theme.c("topbar.bg"))
                    .corner_radius(theme.radius_md())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(width - 24.0);
                        ui.horizontal(|ui| {
                            ui.add(icons::image(
                                ctx,
                                session.tool.icon(),
                                Weight::Regular,
                                18.0,
                                theme.c("topbar.fg"),
                            ));
                            ui.label(
                                RichText::new(plain(texts, session.tool.label()))
                                    .font(theme.font("ui-label"))
                                    .color(theme.c("topbar.fg")),
                            );
                        });
                        ui.label(
                            RichText::new(plain(texts, session.tool.hint()))
                                .font(theme.font("body-sm"))
                                .color(theme.c("topbar.fg.muted")),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(plain(texts, "model3d.hint_keys"))
                                .font(theme.font("caption"))
                                .color(theme.c("topbar.fg.muted")),
                        );
                    });
            });
    }

    orientation_cube(ctx, theme, texts, session, viewport);
    tool_bar(ctx, theme, texts, session, rect, viewport, &mut asks);
    asks
}

fn axis_length(projected: f32, radius: f32) -> f32 {
    // Long enough to grab, never past the element.
    (projected * 1.4)
        .clamp(radius * 0.5, radius * 0.95)
        .max(48.0)
}

fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// The orientation cube at the top right of the canvas: the axes of the model's frame as the
/// camera sees them, the name of the view, and a click on an axis end turns to that view.
fn orientation_cube(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    session: &mut Session,
    viewport: Rect,
) {
    let size = 128.0;
    let at = Pos2::new(viewport.right() - size - 16.0, viewport.top() + 16.0);
    egui::Area::new(egui::Id::new("model3d.cube"))
        .order(egui::Order::Foreground)
        .fixed_pos(at)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
            let painter = ui.painter();
            let c = rect.center();
            painter.circle_filled(c, size / 2.0, theme.c("surface"));
            painter.circle_stroke(c, size / 2.0, Stroke::new(1.0, theme.c("border")));
            let cam = camera_of(&session.current);
            let b = cam.basis();
            let colors = [theme.c("gizmo.x"), theme.c("gizmo.y"), theme.c("gizmo.z")];
            // Each axis end: screen offset, depth, axis, sign.
            let mut ends: Vec<(Vec2, f32, usize, f32)> = Vec::new();
            for axis in 0..3 {
                for sign in [1.0_f32, -1.0] {
                    let mut v = [0.0_f32; 3];
                    v[axis] = sign;
                    let v = glam_vec(v);
                    let screen = Vec2::new(v.dot(b.right), -v.dot(b.up)) * (size * 0.36);
                    ends.push((screen, v.dot(b.back), axis, sign));
                }
            }
            ends.sort_by(|a, b| a.1.total_cmp(&b.1));
            // A little cube in the middle, its faces lit like the model.
            let half = size * 0.14;
            let corners: Vec<(Vec2, f32)> = (0..8)
                .map(|i| {
                    let v = glam_vec([
                        if i & 1 == 0 { -1.0 } else { 1.0 },
                        if i & 2 == 0 { -1.0 } else { 1.0 },
                        if i & 4 == 0 { -1.0 } else { 1.0 },
                    ]);
                    (
                        Vec2::new(v.dot(b.right), -v.dot(b.up)) * half,
                        v.dot(b.back),
                    )
                })
                .collect();
            let faces: [([usize; 4], [f32; 3]); 6] = [
                ([1, 3, 7, 5], [1.0, 0.0, 0.0]),
                ([0, 4, 6, 2], [-1.0, 0.0, 0.0]),
                ([2, 6, 7, 3], [0.0, 1.0, 0.0]),
                ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
                ([4, 5, 7, 6], [0.0, 0.0, 1.0]),
                ([0, 2, 3, 1], [0.0, 0.0, -1.0]),
            ];
            for (idx, normal) in faces {
                let n = glam_vec(normal);
                let facing = n.dot(b.back);
                if facing <= 0.0 {
                    continue;
                }
                let shade = 0.55 + 0.45 * facing;
                let base = theme.c("surface.sunken");
                let fill = Color32::from_rgb(
                    (f32::from(base.r()) * shade) as u8,
                    (f32::from(base.g()) * shade) as u8,
                    (f32::from(base.b()) * shade) as u8,
                );
                let pts: Vec<Pos2> = idx.iter().map(|&i| c + corners[i].0).collect();
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    fill,
                    Stroke::new(1.0, theme.c("border.strong")),
                ));
            }
            let pointer = ui.input(|i| i.pointer.hover_pos());
            let clicked = ui.input(|i| i.pointer.primary_clicked());
            for (screen, depth, axis, sign) in ends {
                let tip = c + screen;
                let color = if depth < -0.05 {
                    colors[axis].gamma_multiply(0.45)
                } else {
                    colors[axis]
                };
                if sign > 0.0 {
                    arrow(painter, c + screen * 0.45, tip, color, 2.5);
                    painter.text(
                        tip + screen.normalized() * 9.0,
                        Align2::CENTER_CENTER,
                        ["X", "Y", "Z"][axis],
                        theme.font("caption"),
                        color,
                    );
                } else {
                    painter.circle_stroke(tip, 4.0, Stroke::new(1.5, color));
                }
                let near = pointer.is_some_and(|p| (p - tip).length() < 11.0);
                if near {
                    painter.circle_stroke(tip, 9.0, Stroke::new(1.5, color));
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if near && clicked {
                    let preset = match (axis, sign > 0.0) {
                        (0, true) => ViewPreset::Right,
                        (0, false) => ViewPreset::Left,
                        (1, true) => ViewPreset::Top,
                        (1, false) => ViewPreset::Bottom,
                        (2, true) => ViewPreset::Front,
                        _ => ViewPreset::Back,
                    };
                    session.set(with_preset(&session.current, preset));
                }
            }
            let name = plain(texts, preset_key(preset_of(&session.current)));
            painter.text(
                Pos2::new(c.x, rect.bottom() - 16.0),
                Align2::CENTER_BOTTOM,
                name,
                theme.font("caption"),
                theme.c("text"),
            );
        });
}

fn glam_vec(v: [f32; 3]) -> glam::Vec3 {
    glam::Vec3::from_array(v)
}

/// The tool bar under the element: tools, views, fit and reset, then Cancel and Apply.
fn tool_bar(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    session: &mut Session,
    rect: Rect,
    viewport: Rect,
    asks: &mut Asks,
) {
    let height = 64.0;
    let below = rect.bottom() + 16.0 + height < viewport.bottom();
    let y = if below {
        rect.bottom() + 16.0
    } else {
        (viewport.bottom() - height - 12.0).max(viewport.top())
    };
    let id = egui::Id::new("model3d.toolbar");
    let width = ctx.memory(|m| m.area_rect(id)).map_or(560.0, |r| r.width());
    // A canvas narrower than the bar keeps it at its left edge.
    let lo = viewport.left() + 8.0;
    let hi = (viewport.right() - width - 8.0).max(lo);
    let x = (rect.center().x - width / 2.0).clamp(lo, hi);
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::new(x, y))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(theme.radius_md())
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for tool in Tool::ALL {
                            if tool_button(
                                ui,
                                theme,
                                tool.icon(),
                                &plain(texts, tool.label()),
                                session.tool == tool,
                            )
                            .clicked()
                            {
                                session.tool = tool;
                            }
                        }
                        ui.separator();
                        let view = tool_button(
                            ui,
                            theme,
                            Icon::Cube,
                            &plain(texts, "model3d.view"),
                            false,
                        );
                        egui::Popup::menu(&view).show(|ui| {
                            for preset in ViewPreset::ALL {
                                let on = preset_of(&session.current) == Some(preset);
                                if ui
                                    .selectable_label(on, plain(texts, preset_key(Some(preset))))
                                    .clicked()
                                {
                                    session.set(with_preset(&session.current, preset));
                                }
                            }
                        });
                        if tool_button(
                            ui,
                            theme,
                            Icon::FitView,
                            &plain(texts, "model3d.fit"),
                            false,
                        )
                        .clicked()
                        {
                            session.set(fitted(&session.current));
                        }
                        if tool_button(ui, theme, Icon::Undo, &plain(texts, "model3d.reset"), false)
                            .clicked()
                        {
                            session.set(session.current.reset());
                        }
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if crate::widgets::secondary(
                                    ui,
                                    theme,
                                    plain(texts, "model3d.cancel"),
                                )
                                .clicked()
                                {
                                    asks.cancel = true;
                                }
                                if crate::widgets::primary(ui, theme, plain(texts, "model3d.apply"))
                                    .clicked()
                                {
                                    asks.apply = true;
                                }
                            });
                        });
                    });
                });
        });
}

fn tool_button(
    ui: &mut egui::Ui,
    theme: &Theme,
    icon: Icon,
    label: &str,
    on: bool,
) -> egui::Response {
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), theme.font("caption"), Color32::WHITE);
    let size = Vec2::new((galley.size().x + 14.0).max(52.0), 50.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter();
    if on {
        painter.rect_filled(rect, theme.radius_md(), theme.c("selection.bg"));
    } else if response.hovered() {
        painter.rect_filled(rect, theme.radius_md(), theme.c("secondary.bg"));
    }
    let color = theme.c("text");
    icons::paint(
        painter,
        Rect::from_center_size(rect.center() - Vec2::new(0.0, 9.0), Vec2::splat(20.0)),
        icon,
        if on { Weight::Fill } else { Weight::Regular },
        color,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - 6.0),
        Align2::CENTER_BOTTOM,
        label,
        theme.font("caption"),
        color,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

// ---------------------------------------------------------------------------------------
// The «Objeto 3D» properties.

/// The lighting presets in the order the panel shows them, with their names.
const LIGHTS: [(Model3dLighting, &str); 6] = [
    (Model3dLighting::Studio, "model3d.studio"),
    (Model3dLighting::Warm, "model3d.warm"),
    (Model3dLighting::Cool, "model3d.cool"),
    (Model3dLighting::Dark, "model3d.dark"),
    (Model3dLighting::Soft, "model3d.soft"),
    (Model3dLighting::Natural, "model3d.natural"),
];

/// The tabs of the properties (spec 100, the panel of the reference pictures).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tab {
    #[default]
    Transform,
    Camera,
    Light,
}

/// What the properties ask for.
pub(crate) enum PanelAsk {
    /// The model changed; `step` closes the undo step (a click, not a slider drag).
    Set(Box<Model3dNode>, bool),
    Edit,
    Apply,
    Cancel,
}

/// The properties of a 3D model: tabs Transformar, Cámara and Luz, and the way into the 3D
/// editor or out of it. `thumb` draws a quick view tile of the model.
pub(crate) fn panel(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    model: &Model3dNode,
    tab: &mut Tab,
    editing: bool,
    thumb: &mut dyn FnMut(&Model3dNode, u8) -> Option<egui::TextureId>,
) -> Option<PanelAsk> {
    let mut ask = None;
    let mut next = model.clone();
    let mut step = true;
    ui.add_space(4.0);
    if editing {
        ui.horizontal(|ui| {
            let half = (ui.available_width() - 8.0) / 2.0;
            if ui
                .add_sized(
                    [half, 34.0],
                    egui::Button::new(plain(texts, "model3d.cancel")),
                )
                .clicked()
            {
                ask = Some(PanelAsk::Cancel);
            }
            let apply = egui::Button::new(
                RichText::new(plain(texts, "model3d.apply")).color(theme.c("primary.fg")),
            )
            .fill(theme.c("primary.bg"));
            if ui.add_sized([half, 34.0], apply).clicked() {
                ask = Some(PanelAsk::Apply);
            }
        });
    } else {
        let edit = egui::Button::image_and_text(
            icons::image(
                ui.ctx(),
                Icon::Cube,
                Weight::Regular,
                18.0,
                theme.c("primary.fg"),
            ),
            RichText::new(plain(texts, "model3d.edit")).color(theme.c("primary.fg")),
        )
        .fill(theme.c("primary.bg"));
        if ui.add_sized([ui.available_width(), 36.0], edit).clicked() {
            ask = Some(PanelAsk::Edit);
        }
    }
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        for (t, key) in [
            (Tab::Transform, "model3d.tab_transform"),
            (Tab::Camera, "model3d.tab_camera"),
            (Tab::Light, "model3d.tab_light"),
        ] {
            let on = *tab == t;
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
            if response.clicked() {
                *tab = t;
            }
            ui.add_space(6.0);
        }
    });
    let deg = slider::degrees(texts);
    let pct = slider::percent(texts);
    match *tab {
        Tab::Transform => {
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.rotation"));
            let mut angles =
                gafira_model3d::euler_degrees_from_quat(pose_of(model).rotation).map(f64::from);
            let mut turned = false;
            for (i, key) in ["model3d.axis_x", "model3d.axis_y", "model3d.axis_z"]
                .into_iter()
                .enumerate()
            {
                if Slider::new(plain(texts, key), &mut angles[i], (-180.0, 180.0))
                    .rest(0.0)
                    .suffix(deg.clone())
                    .show(ui, theme)
                {
                    turned = true;
                    step = false;
                }
            }
            if turned {
                next.rotation = gafira_model3d::quat_from_euler_degrees(angles.map(|a| a as f32))
                    .map(f64::from);
            }
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.size"));
            let mut size = model.scale[0] * 100.0;
            if Slider::new(plain(texts, "model3d.scale"), &mut size, (10.0, 400.0))
                .rest(100.0)
                .suffix(pct.clone())
                .logarithmic()
                .show(ui, theme)
            {
                let s = (size / 100.0).clamp(Model3dNode::MIN_SCALE, Model3dNode::MAX_SCALE);
                next.scale = [s; 3];
                step = false;
            }
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for p in [50.0, 75.0, 100.0, 125.0, 150.0, 200.0] {
                    let on = (model.scale[0] * 100.0 - p).abs() < 0.5;
                    let text = RichText::new(format!("{p:.0}{pct}")).font(theme.font("caption"));
                    let button = egui::Button::new(text)
                        .selected(on)
                        .min_size(Vec2::new(44.0, 26.0));
                    if ui.add(button).clicked() {
                        next.scale = [p / 100.0; 3];
                    }
                }
            });
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.offset"));
            let mut x = model.position[0] * 100.0;
            let mut y = model.position[1] * 100.0;
            if Slider::new(plain(texts, "model3d.offset_x"), &mut x, (-150.0, 150.0))
                .rest(0.0)
                .suffix(pct.clone())
                .show(ui, theme)
            {
                next.position[0] = x / 100.0;
                step = false;
            }
            if Slider::new(plain(texts, "model3d.offset_y"), &mut y, (-150.0, 150.0))
                .rest(0.0)
                .suffix(pct.clone())
                .show(ui, theme)
            {
                next.position[1] = y / 100.0;
                step = false;
            }
            ui.add_space(8.0);
            if ui
                .add_sized(
                    [ui.available_width(), 30.0],
                    egui::Button::new(plain(texts, "model3d.reset")),
                )
                .clicked()
            {
                let reset = model.reset();
                next.rotation = reset.rotation;
                next.position = reset.position;
                next.scale = reset.scale;
            }
        }
        Tab::Camera => {
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.views"));
            let current = preset_of(model);
            let tile = (ui.available_width() - 3.0 * 6.0) / 4.0;
            for row in ViewPreset::ALL.chunks(4) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                    for preset in row {
                        let on = current == Some(*preset);
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(tile, tile + 16.0),
                            egui::Sense::click(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(rect, theme.radius_md(), theme.c("surface"));
                        let stroke = if on {
                            Stroke::new(2.0, theme.c("primary.bg"))
                        } else {
                            Stroke::new(1.0, theme.c("border"))
                        };
                        painter.rect_stroke(
                            rect,
                            theme.radius_md(),
                            stroke,
                            egui::StrokeKind::Inside,
                        );
                        let pic = Rect::from_min_size(
                            rect.min + Vec2::splat(4.0),
                            Vec2::splat(tile - 8.0),
                        );
                        match thumb(&with_preset(model, *preset), *preset as u8) {
                            Some(texture) => {
                                painter.image(
                                    texture,
                                    pic,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }
                            None => icons::paint(
                                painter,
                                pic.shrink(tile * 0.25),
                                Icon::Cube,
                                Weight::Regular,
                                theme.c("text.muted"),
                            ),
                        }
                        painter.text(
                            Pos2::new(rect.center().x, rect.bottom() - 4.0),
                            Align2::CENTER_BOTTOM,
                            plain(texts, preset_key(Some(*preset))),
                            theme.font("caption"),
                            theme.c("text"),
                        );
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if response.clicked() {
                            next = with_preset(&next, *preset);
                        }
                    }
                });
                ui.add_space(6.0);
            }
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.camera"));
            let mut yaw = model.camera.yaw;
            if Slider::new(plain(texts, "model3d.turn"), &mut yaw, (-180.0, 180.0))
                .rest(0.0)
                .suffix(deg.clone())
                .show(ui, theme)
            {
                next.camera.yaw = yaw;
                step = false;
            }
            let mut elevation = model.camera.elevation;
            if Slider::new(
                plain(texts, "model3d.elevation"),
                &mut elevation,
                (-90.0, 90.0),
            )
            .rest(0.0)
            .suffix(deg.clone())
            .show(ui, theme)
            {
                next.camera.elevation = elevation;
                step = false;
            }
            let mut zoom = model.camera.zoom * 100.0;
            if Slider::new(plain(texts, "model3d.distance"), &mut zoom, (20.0, 800.0))
                .rest(100.0)
                .suffix(pct.clone())
                .logarithmic()
                .show(ui, theme)
            {
                next.camera.zoom = zoom / 100.0;
                step = false;
            }
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.projection"));
            ui.horizontal(|ui| {
                for (kind, key) in [
                    (Model3dProjection::Perspective, "model3d.perspective"),
                    (Model3dProjection::Orthographic, "model3d.orthographic"),
                ] {
                    let on = model.camera.projection == kind;
                    if ui
                        .add(egui::Button::new(plain(texts, key)).selected(on))
                        .clicked()
                    {
                        next.camera.projection = kind;
                    }
                }
            });
            if model.camera.projection == Model3dProjection::Perspective {
                let mut fov = model.camera.fov;
                if Slider::new(plain(texts, "model3d.fov"), &mut fov, (10.0, 90.0))
                    .rest(35.0)
                    .suffix(deg)
                    .show(ui, theme)
                {
                    next.camera.fov = fov;
                    step = false;
                }
            }
            ui.add_space(8.0);
            if ui
                .add_sized(
                    [ui.available_width(), 30.0],
                    egui::Button::new(plain(texts, "model3d.reset_view")),
                )
                .clicked()
            {
                next.camera = model.reset().camera;
            }
        }
        Tab::Light => {
            // Six presets in two rows, each tile the model itself under that light.
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.lighting"));
            let tile = (ui.available_width() - 2.0 * 8.0) / 3.0;
            for row in LIGHTS.chunks(3) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                    for (light, key) in row {
                        let on = model.lighting == *light;
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(tile, tile + 18.0),
                            egui::Sense::click(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(rect, theme.radius_md(), theme.c("surface"));
                        let stroke = if on {
                            Stroke::new(2.0, theme.c("primary.bg"))
                        } else if response.hovered() {
                            Stroke::new(1.5, theme.c("border.strong"))
                        } else {
                            Stroke::new(1.0, theme.c("border"))
                        };
                        painter.rect_stroke(
                            rect,
                            theme.radius_md(),
                            stroke,
                            egui::StrokeKind::Inside,
                        );
                        let pic = Rect::from_min_size(
                            rect.min + Vec2::splat(4.0),
                            Vec2::splat(tile - 8.0),
                        );
                        let mut lit = model.clone();
                        lit.lighting = *light;
                        match thumb(&lit, 16 + *light as u8) {
                            Some(texture) => {
                                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                                painter.image(texture, pic, uv, Color32::WHITE);
                            }
                            None => icons::paint(
                                painter,
                                pic.shrink(tile * 0.3),
                                Icon::Light,
                                Weight::Regular,
                                theme.c("text.muted"),
                            ),
                        }
                        painter.text(
                            Pos2::new(rect.center().x, rect.bottom() - 4.0),
                            Align2::CENTER_BOTTOM,
                            plain(texts, key),
                            theme.font("caption"),
                            theme.c("text"),
                        );
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if response.clicked() {
                            next.lighting = *light;
                        }
                    }
                });
                ui.add_space(8.0);
            }
        }
    }
    if ask.is_none() && next != *model && next.is_valid() {
        ask = Some(PanelAsk::Set(Box::new(next), step));
    }
    ask
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_doc::timeline::AssetRef;

    fn model() -> Model3dNode {
        Model3dNode::new(
            AssetRef::new(ContentHash([3; 32])),
            gafira_doc::Model3dFormat::Glb,
        )
    }

    // Spec 100 §3.5: the session keeps its own undo; nothing is modified until it changes.
    #[test]
    fn s100_the_session_undoes_its_own_steps() {
        let mut s = Session::new(
            PageId::new(1),
            NodeId::new(2),
            model(),
            Placement::new(0.0, 0.0, 100.0, 100.0),
        );
        assert!(!s.modified());
        s.set(with_preset(&s.current, ViewPreset::Top));
        s.set(fitted(&s.current));
        assert!(s.modified());
        s.undo();
        s.undo();
        assert!(!s.modified());
        s.redo();
        assert_eq!(preset_of(&s.current), Some(ViewPreset::Top));
        // An invalid model never enters the session.
        let mut broken = s.current.clone();
        broken.scale = [0.0; 3];
        s.set(broken);
        assert_eq!(preset_of(&s.current), Some(ViewPreset::Top));
    }

    // Spec 100 «Escalar»: the frame handles resize the element's box on the page, keeping
    // its proportions; Shift frees an edge; the drag is one step of the session's undo.
    #[test]
    fn s100_the_frame_scales_the_box_in_proportion() {
        let origin = Placement::new(100.0, 100.0, 200.0, 100.0);
        let corner = framed(
            &origin,
            Handle { x: 1, y: 1 },
            Vec2::new(100.0, 0.0),
            1.0,
            false,
        );
        assert_eq!(
            (corner.x, corner.y, corner.w, corner.h),
            (100.0, 100.0, 300.0, 150.0)
        );
        // The right edge: the left stays, the box keeps its proportions, centred across.
        let edge = framed(
            &origin,
            Handle { x: 1, y: 0 },
            Vec2::new(200.0, 0.0),
            2.0,
            false,
        );
        assert_eq!(
            (edge.x, edge.w, edge.h, edge.y),
            (100.0, 300.0, 150.0, 75.0)
        );
        // The top edge pulled down shrinks it about its bottom.
        let top = framed(
            &origin,
            Handle { x: 0, y: -1 },
            Vec2::new(0.0, 50.0),
            1.0,
            false,
        );
        assert_eq!((top.w, top.h, top.y + top.h), (100.0, 50.0, 200.0));
        // With Shift an edge moves only its own side.
        let wide = framed(
            &origin,
            Handle { x: 1, y: 0 },
            Vec2::new(50.0, 0.0),
            1.0,
            true,
        );
        assert_eq!((wide.w, wide.h), (250.0, 100.0));
        // Never below the smallest box.
        let tiny = framed(
            &origin,
            Handle { x: 1, y: 0 },
            Vec2::new(-1000.0, 0.0),
            1.0,
            false,
        );
        assert!(tiny.w >= crate::canvas::MIN_SIDE && tiny.h >= crate::canvas::MIN_SIDE);

        let mut s = Session::new(PageId::new(1), NodeId::new(2), model(), origin);
        s.push_undo();
        s.placement = corner;
        assert!(s.modified() && s.resized());
        s.undo();
        assert_eq!(s.placement, origin);
        assert!(!s.modified());
        s.redo();
        assert_eq!(s.placement, corner);
    }

    // Spec 100 AC-07: a quick view is found again from its angles; fitting recentres.
    #[test]
    fn s100_quick_views_and_fit() {
        for p in ViewPreset::ALL {
            assert_eq!(preset_of(&with_preset(&model(), p)), Some(p));
        }
        let mut m = model();
        m.position = [0.5, 0.2, 0.0];
        m.camera.zoom = 3.0;
        m.scale = [0.4; 3];
        let f = fitted(&m);
        assert_eq!(
            (f.position, f.scale, f.camera.zoom),
            ([0.0; 3], [1.0; 3], 1.0)
        );
    }

    // Spec 100 §3.1: the importer tells models by extension and refuses other files.
    #[test]
    fn s100_model_files_are_told_apart() {
        assert!(is_model_path(Path::new("a/Gift.GLB")));
        assert!(is_model_path(Path::new("b.obj")));
        assert!(!is_model_path(Path::new("c.png")));
        assert_eq!(
            check_file(b"hello", "x.txt").err(),
            Some(ImportError::UnknownFormat)
        );
        let cube = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 2 3\nf 1 2 4\n";
        let (format, warnings) = check_file(cube.as_bytes(), "c.obj").expect("valid");
        assert_eq!(format, ModelFormat::Obj);
        assert!(warnings.is_empty());
    }
}
