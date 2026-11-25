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
use gafira_doc::{ContentHash, Model3dLighting, Model3dNode, Model3dProjection, NodeId, PageId};
use gafira_io_model3d::{
    ImportError, ImportLimits, ImportWarning, ModelDecoder, ModelFormat, camera_of, detect_format,
    params_of, pose_of, render_parallel,
};
use gafira_model3d::{Pose, ViewPreset};
use gafira_ports::export::ImageDecoder;
use gafira_ports::media::MediaError;
use gafira_raster::ImageBuf;

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
    Scale {
        centre: Pos2,
        from: f32,
    },
    Orbit,
}

#[derive(Clone, Debug)]
struct Drag {
    start: Pos2,
    grip: Grip,
    before: Model3dNode,
}

/// An open 3D editing session of one element (spec 100 §3.3, §3.5): its changes stay here
/// until Apply makes them one undo step; Cancel drops them.
#[derive(Clone, Debug)]
pub(crate) struct Session {
    pub(crate) page: PageId,
    pub(crate) node: NodeId,
    pub(crate) original: Model3dNode,
    pub(crate) current: Model3dNode,
    pub(crate) tool: Tool,
    undo: Vec<Model3dNode>,
    redo: Vec<Model3dNode>,
    drag: Option<Drag>,
    /// Where the element was on the screen in the last frame, for the wheel.
    pub(crate) rect: Option<Rect>,
    /// The last change was the wheel: more wheel joins the same step.
    merging: bool,
}

impl Session {
    pub(crate) fn new(page: PageId, node: NodeId, model: Model3dNode) -> Self {
        Self {
            page,
            node,
            original: model.clone(),
            current: model,
            tool: Tool::Rotate,
            undo: Vec::new(),
            redo: Vec::new(),
            drag: None,
            rect: None,
            merging: false,
        }
    }

    pub(crate) fn modified(&self) -> bool {
        self.current != self.original
    }

    pub(crate) fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Sets the model, keeping the step for the session's undo.
    pub(crate) fn set(&mut self, model: Model3dNode) {
        self.merging = false;
        if model != self.current && model.is_valid() {
            self.undo.push(std::mem::replace(&mut self.current, model));
            self.redo.clear();
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
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
        if let Some(previous) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.current, previous));
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.current, next));
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

fn arrow(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32, width: f32) {
    painter.line_segment([from, to], Stroke::new(width, color));
    let dir = (to - from).normalized();
    if !dir.is_finite() {
        return;
    }
    let side = Vec2::new(-dir.y, dir.x);
    let head = 4.0 * width;
    painter.add(egui::Shape::convex_polygon(
        vec![
            to + dir * head * 0.6,
            to - dir * head * 0.6 + side * head * 0.55,
            to - dir * head * 0.6 - side * head * 0.55,
        ],
        color,
        Stroke::NONE,
    ));
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

/// The gizmos, the orientation cube, the hint and the tool bar of the session, over the
/// element at `rect` (screen) on a canvas whose visible area is `viewport`.
pub(crate) fn overlay(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    session: &mut Session,
    rect: Rect,
    viewport: Rect,
) -> Asks {
    let mut asks = Asks::default();
    session.rect = Some(rect);
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("model3d.gizmos"));
    let painter = ctx.layer_painter(layer).with_clip_rect(viewport);
    let accent = rgb(theme, "primary.bg");
    let (gx, gy, gz) = (
        rgb(theme, "gizmo.x"),
        rgb(theme, "gizmo.y"),
        rgb(theme, "gizmo.z"),
    );

    // The surface that takes the gestures: nothing behind it moves (spec 100 §3.4).
    let area = egui::Area::new(egui::Id::new("model3d.surface"))
        .order(egui::Order::Middle)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.allocate_exact_size(rect.size(), egui::Sense::click_and_drag())
                .1
        });
    let response = area.inner;
    let pointer = ctx.input(|i| i.pointer.interact_pos());

    let model = session.current.clone();
    let centre = project(&model, rect, model.position.map(|v| v as f32)).unwrap_or(rect.center());
    let radius = rect.width().min(rect.height()) * 0.42;
    let (ring_rx, ring_ry) = (radius, radius * 0.3);

    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.5, accent),
        egui::StrokeKind::Outside,
    );

    // What a press grabs, by tool and place.
    let tool = session.tool;
    let grip_at = |p: Pos2| -> Grip {
        match tool {
            Tool::Rotate => {
                if near_ellipse(p, centre, ring_rx, ring_ry, 10.0) {
                    Grip::Yaw
                } else if near_ellipse(p, centre, ring_ry, ring_rx, 10.0) {
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
                        if ends < 10.0 && best.is_none_or(|(b, _)| ends < b) {
                            best = Some((ends, Grip::Axis(axis, d * 2.0)));
                        }
                    }
                }
                best.map_or(Grip::Plane, |(_, g)| g)
            }
            Tool::Scale => Grip::Scale {
                centre,
                from: (p - centre).length().max(8.0),
            },
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
            Grip::Scale { centre, from } => {
                let factor = ((p - centre).length() / from).clamp(0.05, 20.0);
                let mut scaled = pose;
                scaled.scale = pose.scale.map(|s| s * factor);
                to_doc_pose(&mut next, scaled);
            }
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
        && drag.before != session.current
    {
        // The whole drag is one step of the session's undo.
        let done = std::mem::replace(&mut session.current, drag.before);
        session.set(done);
    }
    if response.hovered() {
        ctx.set_cursor_icon(if session.drag.is_some() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    }

    // The gizmo of the tool.
    let hover = pointer.filter(|p| rect.contains(*p));
    let hovered_grip = if session.drag.is_none() {
        hover.map(grip_at)
    } else {
        session.drag.as_ref().map(|d| d.grip)
    };
    let model = session.current.clone();
    match session.tool {
        Tool::Rotate => {
            let yaw_on = matches!(hovered_grip, Some(Grip::Yaw));
            let pitch_on = matches!(hovered_grip, Some(Grip::Pitch));
            painter.add(egui::Shape::line(
                ellipse_points(centre, ring_ry, ring_rx, 0.0, std::f32::consts::TAU),
                Stroke::new(if pitch_on { 5.0 } else { 3.0 }, gy),
            ));
            painter.add(egui::Shape::line(
                ellipse_points(centre, ring_rx, ring_ry, 0.0, std::f32::consts::TAU),
                Stroke::new(if yaw_on { 5.0 } else { 3.0 }, gz),
            ));
            arrow(
                &painter,
                centre + Vec2::new(ring_rx * 0.7, ring_ry * 0.72),
                centre + Vec2::new(ring_rx * 0.95, ring_ry * 0.3),
                gz,
                3.0,
            );
            arrow(
                &painter,
                centre + Vec2::new(ring_ry * 0.72, -ring_rx * 0.7),
                centre + Vec2::new(ring_ry * 0.3, -ring_rx * 0.95),
                gy,
                3.0,
            );
            painter.circle_filled(centre, 4.0, Color32::WHITE);
            painter.circle_stroke(centre, 4.0, Stroke::new(1.5, accent));
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
            for (axis, d, _) in axes {
                if d.length() < 4.0 {
                    continue;
                }
                let color = [gx, gy, gz][axis];
                let on = matches!(hovered_grip, Some(Grip::Axis(a, _)) if a == axis);
                let tip = centre + d.normalized() * axis_length(d.length(), radius);
                arrow(&painter, centre, tip, color, if on { 5.0 } else { 3.5 });
            }
            painter.circle_filled(centre, 7.0, Color32::WHITE);
            painter.circle_stroke(centre, 7.0, Stroke::new(2.0, accent));
        }
        Tool::Scale => {
            let dashed = rect.expand(10.0);
            for (corner, dir) in [
                (dashed.left_top(), Vec2::new(-1.0, -1.0)),
                (dashed.right_top(), Vec2::new(1.0, -1.0)),
                (dashed.left_bottom(), Vec2::new(-1.0, 1.0)),
                (dashed.right_bottom(), Vec2::new(1.0, 1.0)),
            ] {
                arrow(
                    &painter,
                    corner - dir * 6.0,
                    corner + dir * 18.0,
                    accent,
                    3.0,
                );
            }
            painter.add(egui::Shape::dashed_line(
                &[
                    dashed.left_top(),
                    dashed.right_top(),
                    dashed.right_bottom(),
                    dashed.left_bottom(),
                    dashed.left_top(),
                ],
                Stroke::new(1.0, accent),
                6.0,
                4.0,
            ));
            let pct = model.scale[0] * 100.0;
            if session.drag.is_some() {
                badge(
                    &painter,
                    theme,
                    Pos2::new(rect.center().x, rect.min.y - 22.0),
                    &format!("{pct:.0}{}", slider::percent(texts)),
                    accent,
                );
            }
        }
        Tool::Camera => {
            let c = rect.center();
            let (rx, ry) = (rect.width() * 0.5, rect.height() * 0.13);
            let base = Pos2::new(c.x, c.y + rect.height() * 0.22);
            painter.add(egui::Shape::line(
                ellipse_points(base, rx, ry, 0.2, std::f32::consts::PI - 0.2),
                Stroke::new(3.0, gz),
            ));
            arrow(
                &painter,
                base + Vec2::new(rx * 0.55, ry * 0.83),
                base + Vec2::new(rx * 0.8, ry * 0.6),
                gz,
                3.0,
            );
            let arc_c = Pos2::new(c.x, c.y);
            let arc_r = rect.height().min(rect.width()) * 0.48;
            painter.add(egui::Shape::line(
                ellipse_points(arc_c, arc_r, arc_r, -1.35, -0.25),
                Stroke::new(3.0, accent),
            ));
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
    projected.clamp(radius * 0.35, radius * 0.9).max(28.0)
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
    thumb: &mut dyn FnMut(ViewPreset) -> Option<egui::TextureId>,
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
                        match thumb(*preset) {
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
            crate::image_panel::heading(ui, theme, plain(texts, "model3d.lighting"));
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 12.0) / 3.0;
                for (light, key, icon) in [
                    (Model3dLighting::Soft, "model3d.soft", Icon::Light),
                    (
                        Model3dLighting::Studio,
                        "model3d.studio",
                        Icon::CategoryLights,
                    ),
                    (Model3dLighting::Dramatic, "model3d.dramatic", Icon::Dark),
                ] {
                    let on = model.lighting == light;
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(w, 64.0), egui::Sense::click());
                    let painter = ui.painter();
                    painter.rect_filled(
                        rect,
                        theme.radius_md(),
                        if on {
                            theme.c("selection.bg")
                        } else {
                            theme.c("surface")
                        },
                    );
                    let stroke = if on {
                        Stroke::new(2.0, theme.c("primary.bg"))
                    } else {
                        Stroke::new(1.0, theme.c("border"))
                    };
                    painter.rect_stroke(rect, theme.radius_md(), stroke, egui::StrokeKind::Inside);
                    icons::paint(
                        painter,
                        Rect::from_center_size(
                            rect.center() - Vec2::new(0.0, 9.0),
                            Vec2::splat(22.0),
                        ),
                        icon,
                        Weight::Regular,
                        theme.c("text"),
                    );
                    painter.text(
                        Pos2::new(rect.center().x, rect.bottom() - 8.0),
                        Align2::CENTER_BOTTOM,
                        plain(texts, key),
                        theme.font("caption"),
                        theme.c("text"),
                    );
                    if response.clicked() {
                        next.lighting = light;
                    }
                }
            });
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
        let mut s = Session::new(PageId::new(1), NodeId::new(2), model());
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
