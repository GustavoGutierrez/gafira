//! Cropping an image (spec 088): the frame over the whole image, its grips,
//! its proportions and its shapes. Nothing changes in the document until it
//! is applied.

use egui::{Color32, Pos2, Rect, Response, Stroke, Ui, Vec2};
use gafira_doc::{CropShape, ImageCrop, Placement};
use kurbo::Affine;

use crate::canvas;
use crate::rotation;
use crate::style::Theme;

/// The proportions offered: their name and what they are. The first two are
/// «Libre» and the one of the image.
pub(crate) const RATIOS: [(&str, Option<f64>); 9] = [
    ("image.ratio_free", None),
    ("image.ratio_original", None),
    ("1:1", Some(1.0)),
    ("4:3", Some(4.0 / 3.0)),
    ("3:4", Some(3.0 / 4.0)),
    ("3:2", Some(3.0 / 2.0)),
    ("2:3", Some(2.0 / 3.0)),
    ("16:9", Some(16.0 / 9.0)),
    ("9:16", Some(9.0 / 16.0)),
];

/// How near a grip the pointer takes it, in px of the screen.
const NEAR: f32 = 14.0;

/// The least side of the frame, in px of the page.
const LEAST: f64 = 4.0;

/// How dark what does not show is.
const DARK: f32 = 0.62;

/// What is held: the sides it moves, −1, 0 or 1 across and down; all of the
/// frame with none.
type Grip = (i8, i8);

const GRIPS: [Grip; 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
];

/// A frame inside an image, in px: its first corner and its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Frame {
    fn of(crop: &ImageCrop, (w, h): (f64, f64)) -> Self {
        Self {
            x: crop.x * w,
            y: crop.y * h,
            w: crop.w * w,
            h: crop.h * h,
        }
    }

    /// As shares of an image of `w` × `h`, inside it.
    fn put(self, crop: &mut ImageCrop, (w, h): (f64, f64)) {
        let share = |from: f64, size: f64, whole: f64| {
            let size = (size / whole).clamp(ImageCrop::LEAST, 1.0);
            ((from / whole).clamp(0.0, 1.0 - size), size)
        };
        (crop.x, crop.w) = share(self.x, self.w, w);
        (crop.y, crop.h) = share(self.y, self.h, h);
    }
}

/// One side of the frame, `from` and `size` of `whole`, moved by `by` at the
/// end `grip` says.
fn moved((from, size): (f64, f64), whole: f64, grip: i8, by: f64, least: f64) -> (f64, f64) {
    let end = from + size;
    match grip {
        -1 => {
            let from = (from + by).clamp(0.0, (end - least).max(0.0));
            (from, end - from)
        }
        1 => {
            let end = (end + by).clamp((from + least).min(whole), whole);
            (from, end - from)
        }
        _ => (from, size),
    }
}

/// `start` after the grip moved by `by`, inside an image of `size`, keeping
/// `ratio` when there is one.
pub(crate) fn dragged(
    start: Frame,
    grip: Grip,
    by: (f64, f64),
    ratio: Option<f64>,
    size: (f64, f64),
) -> Frame {
    if grip == (0, 0) {
        return Frame {
            x: (start.x + by.0).clamp(0.0, (size.0 - start.w).max(0.0)),
            y: (start.y + by.1).clamp(0.0, (size.1 - start.h).max(0.0)),
            ..start
        };
    }
    let least = LEAST.min(size.0).min(size.1);
    let (x, w) = moved((start.x, start.w), size.0, grip.0, by.0, least);
    let (y, h) = moved((start.y, start.h), size.1, grip.1, by.1, least);
    let Some(ratio) = ratio else {
        return Frame { x, y, w, h };
    };
    // The room there is from what stays: the other side, or the middle.
    let room = |grip: i8, from: f64, length: f64, whole: f64| match grip {
        -1 => from + length,
        1 => whole - from,
        _ => 2.0 * (from + length / 2.0).min(whole - from - length / 2.0),
    };
    let most =
        room(grip.0, start.x, start.w, size.0).min(room(grip.1, start.y, start.h, size.1) * ratio);
    let asked = match grip {
        (0, _) => h * ratio,
        (_, 0) => w,
        _ => w.max(h * ratio),
    };
    let smallest = least.max(least * ratio).min(most);
    let w = asked.clamp(smallest, most);
    let h = w / ratio;
    let place = |grip: i8, from: f64, length: f64, now: f64| match grip {
        -1 => from + length - now,
        1 => from,
        _ => from + (length - now) / 2.0,
    };
    Frame {
        x: place(grip.0, start.x, start.w, w),
        y: place(grip.1, start.y, start.h, h),
        w,
        h,
    }
}

/// The largest frame of `ratio` in an image of `size`, its middle as near as
/// it can to the one of `now`.
pub(crate) fn fitted(now: Frame, ratio: f64, size: (f64, f64)) -> Frame {
    let w = size.0.min(size.1 * ratio);
    let h = w / ratio;
    let middle = (now.x + now.w / 2.0, now.y + now.h / 2.0);
    Frame {
        x: (middle.0 - w / 2.0).clamp(0.0, (size.0 - w).max(0.0)),
        y: (middle.1 - h / 2.0).clamp(0.0, (size.1 - h).max(0.0)),
        w,
        h,
    }
}

/// The part a line drawn by hand, in shares of the image, keeps; none when
/// it is next to nothing.
pub(crate) fn drawn(line: &[[f64; 2]]) -> Option<ImageCrop> {
    let points: Vec<[f64; 2]> = line.iter().map(|p| p.map(|v| v.clamp(0.0, 1.0))).collect();
    let (mut low, mut high) = ([1.0_f64; 2], [0.0_f64; 2]);
    for p in &points {
        for i in 0..2 {
            low[i] = low[i].min(p[i]);
            high[i] = high[i].max(p[i]);
        }
    }
    let (w, h) = (high[0] - low[0], high[1] - low[1]);
    if points.len() < 3 || !(w > 0.01 && h > 0.01) {
        return None;
    }
    // As many as there may be, evenly.
    let step = points.len().div_ceil(ImageCrop::MAX_POINTS).max(1);
    let shape: Vec<[f64; 2]> = points
        .iter()
        .step_by(step)
        .map(|p| {
            [
                ((p[0] - low[0]) / w).clamp(0.0, 1.0),
                ((p[1] - low[1]) / h).clamp(0.0, 1.0),
            ]
        })
        .collect();
    let crop = ImageCrop {
        x: low[0],
        y: low[1],
        w,
        h,
        shape: CropShape::Path(shape),
    };
    crop.is_valid().then_some(crop)
}

/// The image being cropped.
pub(crate) struct Cropper {
    pub crop: ImageCrop,
    /// One of [`RATIOS`].
    pub ratio: usize,
    /// The next drag draws the shape.
    pub drawing: bool,
    held: Option<(Grip, Frame, Pos2)>,
    line: Vec<[f64; 2]>,
    /// The dark around the shape, for the shape it was made of.
    veil: Option<(CropShape, egui::TextureHandle)>,
}

/// What the pointer asked for over the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Done {
    Apply,
}

impl Cropper {
    pub fn new(crop: ImageCrop) -> Self {
        Self {
            crop,
            ratio: 0,
            drawing: false,
            held: None,
            line: Vec::new(),
            veil: None,
        }
    }

    /// The proportion that is kept, of an image of `size`.
    fn kept(&self, size: (f64, f64)) -> Option<f64> {
        match RATIOS.get(self.ratio)? {
            ("image.ratio_original", _) => Some(size.0 / size.1.max(f64::EPSILON)),
            (_, ratio) => *ratio,
        }
    }

    /// Keeps one of [`RATIOS`] from now on.
    pub fn keep(&mut self, ratio: usize, size: (f64, f64)) {
        self.ratio = ratio;
        if let Some(kept) = self.kept(size) {
            fitted(Frame::of(&self.crop, size), kept, size).put(&mut self.crop, size);
        }
    }

    /// Cuts to `shape`: a circle is of one proportion, and a shape drawn by
    /// hand has the one it is drawn with.
    pub fn cut(&mut self, shape: CropShape, size: (f64, f64)) {
        self.drawing = matches!(shape, CropShape::Path(_));
        if self.drawing {
            self.ratio = 0;
            return;
        }
        if shape == CropShape::Ellipse && self.crop.shape != CropShape::Ellipse {
            self.keep(2, size);
        }
        self.crop.shape = shape;
    }

    /// All of the image shows.
    pub fn whole(&mut self) {
        self.crop = ImageCrop::default();
        self.ratio = 0;
        self.drawing = false;
    }

    /// The whole image, what does not show dark, and the frame with its
    /// grips; `picture` is the texture of the whole image.
    pub fn overlay(
        &mut self,
        ui: &Ui,
        painter: &egui::Painter,
        theme: &Theme,
        (response, turn, whole): (&Response, Affine, &Placement),
        picture: Option<egui::TextureId>,
    ) -> Option<Done> {
        let size = (whole.w, whole.h);
        let at = |x: f64, y: f64| rotation::screen(turn, x, y);
        self.point(ui, response, turn, size);
        if let Some(picture) = picture {
            let white = Color32::WHITE;
            painter.add(canvas::image_mesh(picture, turn, whole, white));
        }
        let f = Frame::of(&self.crop, size);
        let ink = theme.c("topbar.bg").gamma_multiply(DARK);
        let quad = |x0: f64, y0: f64, x1: f64, y1: f64| {
            let corners = vec![at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)];
            painter.add(egui::Shape::convex_polygon(corners, ink, Stroke::NONE));
        };
        let (x1, y1) = (f.x + f.w, f.y + f.h);
        quad(0.0, 0.0, size.0, f.y);
        quad(0.0, y1, size.0, size.1);
        quad(0.0, f.y, f.x, y1);
        quad(x1, f.y, size.0, y1);
        if self.crop.shape != CropShape::Rect {
            let veil = self.veil(ui.ctx());
            let part = self.crop.part_box(whole);
            let shown = turn_of(turn, whole, &part);
            painter.add(canvas::image_mesh(veil, shown, &part, ink));
        }
        let mint = theme.c("accent");
        let line = |a: Pos2, b: Pos2, stroke: Stroke| {
            painter.line_segment([a, b], stroke);
        };
        // Thirds, to place what is seen.
        let thin = Stroke::new(1.0, Color32::from_white_alpha(110));
        for n in [1.0, 2.0] {
            let (x, y) = (f.x + f.w * n / 3.0, f.y + f.h * n / 3.0);
            line(at(x, f.y), at(x, y1), thin);
            line(at(f.x, y), at(x1, y), thin);
        }
        let corners = [at(f.x, f.y), at(x1, f.y), at(x1, y1), at(f.x, y1)];
        for n in 0..4 {
            line(corners[n], corners[(n + 1) % 4], Stroke::new(2.0, mint));
        }
        for grip in GRIPS {
            let p = self.grip_at(turn, size, grip);
            let radius = if grip.0 != 0 && grip.1 != 0 { 7.0 } else { 5.5 };
            painter.circle(p, radius, theme.c("surface"), Stroke::new(2.0, mint));
        }
        if self.line.len() > 1 {
            let points = self.line.iter();
            let points = points.map(|p| at(p[0] * size.0, p[1] * size.1)).collect();
            painter.add(egui::Shape::line(points, Stroke::new(2.5, mint)));
        }
        let inside = response.double_clicked() && !self.drawing;
        inside.then_some(Done::Apply)
    }

    fn grip_at(&self, turn: Affine, size: (f64, f64), grip: Grip) -> Pos2 {
        let f = Frame::of(&self.crop, size);
        let place =
            |grip: i8, from: f64, length: f64| from + length * (f64::from(grip) + 1.0) / 2.0;
        rotation::screen(turn, place(grip.0, f.x, f.w), place(grip.1, f.y, f.h))
    }

    /// The grip under a point of the screen, or all of the frame.
    fn grip_under(&self, turn: Affine, size: (f64, f64), p: Pos2) -> Option<Grip> {
        let near = GRIPS
            .into_iter()
            .map(|grip| (grip, self.grip_at(turn, size, grip).distance(p)))
            .filter(|(_, far)| *far <= NEAR)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((grip, _)) = near {
            return Some(grip);
        }
        let f = Frame::of(&self.crop, size);
        let at = rotation::local(turn, p)?;
        let inside = at.x >= f.x && at.y >= f.y && at.x <= f.x + f.w && at.y <= f.y + f.h;
        inside.then_some((0, 0))
    }

    /// What the pointer does.
    fn point(&mut self, ui: &Ui, response: &Response, turn: Affine, size: (f64, f64)) {
        let (latest, pressed, down) = ui.input(|i| {
            let p = &i.pointer;
            (p.latest_pos(), p.press_origin(), p.primary_down())
        });
        let held = response.is_pointer_button_down_on() && down;
        if self.drawing {
            let at = latest.and_then(|p| rotation::local(turn, p));
            if held && let Some(at) = at {
                let p = [at.x / size.0, at.y / size.1].map(|v| v.clamp(0.0, 1.0));
                if self.line.last() != Some(&p) {
                    self.line.push(p);
                }
                ui.ctx().request_repaint();
            } else if !self.line.is_empty() {
                if let Some(crop) = drawn(&std::mem::take(&mut self.line)) {
                    self.crop = crop;
                    // Drawn: the next drag moves it.
                    self.drawing = false;
                }
                self.line.clear();
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            return;
        }
        if held
            && self.held.is_none()
            && let Some(from) = pressed
            && let Some(grip) = self.grip_under(turn, size, from)
        {
            self.held = Some((grip, Frame::of(&self.crop, size), from));
        }
        if !held {
            self.held = None;
        }
        if let Some((grip, start, from)) = self.held
            && let Some(to) = latest
            && let (Some(a), Some(b)) = (rotation::local(turn, from), rotation::local(turn, to))
        {
            let by = (b.x - a.x, b.y - a.y);
            let ratio = self.kept(size);
            dragged(start, grip, by, ratio, size).put(&mut self.crop, size);
            ui.ctx().request_repaint();
        }
        let over = self.held.map(|held| held.0).or_else(|| {
            let p = response.hover_pos()?;
            self.grip_under(turn, size, p)
        });
        if let Some(grip) = over {
            ui.ctx().set_cursor_icon(match grip {
                (0, 0) if self.held.is_some() => egui::CursorIcon::Grabbing,
                (0, 0) => egui::CursorIcon::Grab,
                (0, _) => egui::CursorIcon::ResizeVertical,
                (_, 0) => egui::CursorIcon::ResizeHorizontal,
                (a, b) if a == b => egui::CursorIcon::ResizeNwSe,
                _ => egui::CursorIcon::ResizeNeSw,
            });
        }
    }

    /// White on what the shape leaves out.
    fn veil(&mut self, ctx: &egui::Context) -> egui::TextureId {
        if let Some((of, texture)) = &self.veil
            && *of == self.crop.shape
        {
            return texture.id();
        }
        const SIDE: u32 = 384;
        let cover = gafira_raster::look::coverage(&self.crop.shape, SIDE, SIDE);
        let image = crate::looks::white(&cover, (SIDE, SIDE), true);
        let texture = ctx.load_texture("image.crop", image, egui::TextureOptions::LINEAR);
        let id = texture.id();
        self.veil = Some((self.crop.shape.clone(), texture));
        id
    }
}

/// The transform of the box `part` on the screen, when the one of `whole` is
/// `turn`.
fn turn_of(turn: Affine, whole: &Placement, part: &Placement) -> Affine {
    turn * whole.to_transform().inverse() * part.to_transform()
}

/// The picture of a shape, for its button.
pub(crate) fn shape_icon(painter: &egui::Painter, rect: Rect, shape: &CropShape, color: Color32) {
    let stroke = Stroke::new(2.0, color);
    let r = rect.shrink(2.0);
    let outside = egui::StrokeKind::Middle;
    match shape {
        CropShape::Rect => {
            painter.rect_stroke(r, 0.0, stroke, outside);
        }
        CropShape::Rounded(_) => {
            painter.rect_stroke(r, r.width() * 0.3, stroke, outside);
        }
        CropShape::Ellipse => {
            painter.circle_stroke(r.center(), r.width() / 2.0, stroke);
        }
        CropShape::Path(_) => {
            let points = [
                (0.1, 0.35),
                (0.35, 0.05),
                (0.65, 0.2),
                (0.95, 0.3),
                (0.8, 0.65),
                (0.85, 0.95),
                (0.45, 0.8),
                (0.1, 0.9),
                (0.2, 0.6),
            ];
            let points = points.map(|(x, y)| r.min + Vec2::new(x * r.width(), y * r.height()));
            painter.add(egui::Shape::closed_line(points.to_vec(), stroke));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const SIZE: (f64, f64) = (300.0, 200.0);

    fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
        Frame { x, y, w, h }
    }

    // S88-04: a side moves alone, and stops at the image and at the least.
    #[test]
    fn s88_04_a_side_moves() {
        let start = frame(50.0, 40.0, 100.0, 80.0);
        let right = |by| dragged(start, (1, 0), (by, 999.0), None, SIZE);
        assert_eq!(right(30.0), frame(50.0, 40.0, 130.0, 80.0));
        assert_eq!(right(900.0), frame(50.0, 40.0, 250.0, 80.0));
        assert_eq!(right(-900.0), frame(50.0, 40.0, LEAST, 80.0));
        let corner = dragged(start, (-1, -1), (-20.0, 10.0), None, SIZE);
        assert_eq!(corner, frame(30.0, 50.0, 120.0, 70.0));
        let all = dragged(start, (0, 0), (900.0, -900.0), None, SIZE);
        assert_eq!(all, frame(200.0, 0.0, 100.0, 80.0));
    }

    // S88-04: a proportion chosen is as large as it fits, about where it was.
    #[test]
    fn s88_04_a_proportion() {
        let now = frame(0.0, 0.0, 60.0, 60.0);
        assert_eq!(fitted(now, 1.0, SIZE), frame(0.0, 0.0, 200.0, 200.0));
        let wide = fitted(frame(200.0, 100.0, 60.0, 60.0), 3.0, SIZE);
        assert_eq!(wide, frame(0.0, 80.0, 300.0, 100.0));
        let mut cropper = Cropper::new(ImageCrop::default());
        cropper.cut(CropShape::Ellipse, SIZE);
        let f = Frame::of(&cropper.crop, SIZE);
        assert!((f.w - f.h).abs() < 1e-9, "a circle");
        cropper.keep(1, SIZE);
        let f = Frame::of(&cropper.crop, SIZE);
        assert!((f.w / f.h - 1.5).abs() < 1e-9, "the one of the image");
        cropper.whole();
        assert!(cropper.crop.is_whole());
    }

    // S88-04: a line drawn by hand is the shape, in the box around it; next
    // to nothing is none.
    #[test]
    fn s88_04_drawn_by_hand() {
        let line = [[0.2, 0.2], [0.6, 0.2], [0.6, 0.7], [-3.0, 0.7]];
        let crop = drawn(&line).expect("a shape");
        assert!((crop.x, crop.y) == (0.0, 0.2) && (crop.w - 0.6).abs() < 1e-9);
        let CropShape::Path(points) = &crop.shape else {
            panic!("a path")
        };
        assert_eq!((points.len(), points[3]), (4, [0.0, 1.0]));
        assert!(drawn(&line[..2]).is_none() && drawn(&[[0.5, 0.5]; 40]).is_none());
        let long: Vec<[f64; 2]> = (0..5000)
            .map(|n| {
                let a = f64::from(n) / 795.0;
                [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
            })
            .collect();
        assert!(drawn(&long).is_some_and(|crop| crop.is_valid()));
    }

    proptest! {
        // S88-04: whatever is dragged, the frame stays inside, no smaller
        // than the least, of its proportion, and what is not held stays.
        #[test]
        fn s88_04_any_drag(grip in 0_usize..9, by in (-900.0..900.0_f64, -900.0..900.0_f64), x in 0.0..0.6_f64, y in 0.0..0.6_f64, w in 0.1..0.4_f64, ratio in 0_usize..9) {
            let mut cropper = Cropper::new(ImageCrop { x, y, w, h: w, shape: CropShape::Rect });
            cropper.ratio = ratio;
            let kept = cropper.kept(SIZE);
            let mut start = Frame::of(&cropper.crop, SIZE);
            if let Some(kept) = kept {
                start.h = start.w / kept;
                start.y = start.y.min(SIZE.1 - start.h);
                prop_assume!(start.y >= 0.0 && start.h >= LEAST);
            }
            let grip = GRIPS.get(grip).copied().unwrap_or((0, 0));
            let f = dragged(start, grip, by, kept, SIZE);
            prop_assert!(f.x >= -1e-6 && f.y >= -1e-6 && f.x + f.w <= SIZE.0 + 1e-6 && f.y + f.h <= SIZE.1 + 1e-6, "{f:?}");
            prop_assert!(f.w >= LEAST - 1e-6 && f.h >= LEAST - 1e-6, "{f:?}");
            if let Some(kept) = kept {
                prop_assert!((f.w / f.h - kept).abs() < 1e-6, "{f:?}");
            }
            if grip.0 == 1 { prop_assert!((f.x - start.x).abs() < 1e-6); }
            if grip.0 == -1 { prop_assert!((f.x + f.w - start.x - start.w).abs() < 1e-6); }
            if grip.1 == 1 { prop_assert!((f.y - start.y).abs() < 1e-6); }
            if grip.1 == -1 { prop_assert!((f.y + f.h - start.y - start.h).abs() < 1e-6); }
            f.put(&mut cropper.crop, SIZE);
            prop_assert!(cropper.crop.is_valid(), "{:?}", cropper.crop);
        }
    }
}
