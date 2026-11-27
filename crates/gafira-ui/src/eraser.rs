//! «Borrador de píxeles» (spec 084, S84-08): erasing and bringing back the
//! pixels of an image, over a copy of its opacity. The image itself does not
//! change until «Listo».

use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::{ImageNode, NodeId, PageId};
use gafira_ports::vision::Pixels;

use crate::charts::plain;
use crate::editor::{bar_separator, zoom_controls};
use crate::style::{Theme, px};
use crate::text::Texts;
use crate::widgets;
use crate::zoom::{self, Zoom, ZoomAsk};

/// What the brush does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Erase,
    Restore,
}

/// The sizes of the brush, in px of the image.
pub(crate) const SIZES: std::ops::RangeInclusive<f32> = 1.0..=500.0;

/// The share of the brush that is not soft.
const HARD: f32 = 0.8;

/// The most strokes that can be undone, and the most bytes they take.
const MOST_UNDO: usize = 60;
const MOST_UNDO_BYTES: usize = 200 << 20;

/// A part of the opacity, row by row.
#[derive(Clone, Debug, PartialEq)]
struct Patch {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    alpha: Vec<u8>,
}

/// The opacity of an image being painted, its strokes undone and done again.
#[derive(Clone, Debug)]
pub(crate) struct Paint {
    width: u32,
    height: u32,
    alpha: Vec<u8>,
    /// The opacity the image has with nothing cut out.
    whole: Vec<u8>,
    /// The opacity when the stroke began, while there is one.
    before: Option<Vec<u8>>,
    touched: Option<Bounds>,
    undo: Vec<Patch>,
    redo: Vec<Patch>,
}

/// Pixels from the first, up to the last not included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl Bounds {
    fn with(self, other: Self) -> Self {
        Self {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }
}

impl Paint {
    /// None when the opacities are not of `width` × `height`.
    pub fn new((width, height): (u32, u32), alpha: Vec<u8>, whole: Vec<u8>) -> Option<Self> {
        let count = (width as usize).checked_mul(height as usize)?;
        (alpha.len() == count && whole.len() == count && count > 0).then_some(Self {
            width,
            height,
            alpha,
            whole,
            before: None,
            touched: None,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    pub fn alpha(&self) -> &[u8] {
        &self.alpha
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// A stroke begins.
    pub fn begin(&mut self) {
        self.before = Some(self.alpha.clone());
        self.touched = None;
    }

    /// The brush goes from `a` to `b`, in px of the image; the pixels that
    /// may have changed.
    pub fn line(&mut self, mode: Mode, a: (f32, f32), b: (f32, f32), size: f32) -> Option<Bounds> {
        let radius = (size / 2.0).max(0.5);
        let all = [a.0, a.1, b.0, b.1, radius];
        if all.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let low = |v: f32| (v - radius - 1.0).floor().max(0.0) as u32;
        let high = |v: f32, most: u32| ((v + radius + 2.0).max(0.0) as u32).min(most);
        let bounds = Bounds {
            x0: low(a.0.min(b.0)),
            y0: low(a.1.min(b.1)),
            x1: high(a.0.max(b.0), self.width),
            y1: high(a.1.max(b.1), self.height),
        };
        if bounds.x0 >= bounds.x1 || bounds.y0 >= bounds.y1 {
            return None;
        }
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx * dx + dy * dy;
        for y in bounds.y0..bounds.y1 {
            for x in bounds.x0..bounds.x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let t = if length > 0.0 {
                    (((px - a.0) * dx + (py - a.1) * dy) / length).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (ox, oy) = (px - a.0 - t * dx, py - a.1 - t * dy);
                let cover = cover((ox * ox + oy * oy).sqrt(), radius);
                if cover <= 0.0 {
                    continue;
                }
                let at = y as usize * self.width as usize + x as usize;
                let whole = f32::from(self.whole[at]);
                // Within 0..=255: the casts are exact.
                self.alpha[at] = match mode {
                    Mode::Erase => self.alpha[at].min((whole * (1.0 - cover)).round() as u8),
                    Mode::Restore => self.alpha[at].max((whole * cover).round() as u8),
                };
            }
        }
        self.touched = Some(self.touched.map_or(bounds, |t| t.with(bounds)));
        Some(bounds)
    }

    /// The stroke ends: it can be undone.
    pub fn end(&mut self) {
        let (Some(before), Some(bounds)) = (self.before.take(), self.touched.take()) else {
            return;
        };
        let patch = self.patch(&before, bounds);
        if patch.alpha == self.patch(&self.alpha, bounds).alpha {
            return;
        }
        self.redo.clear();
        self.undo.push(patch);
        let bytes = |list: &[Patch]| list.iter().map(|p| p.alpha.len()).sum::<usize>();
        while self.undo.len() > MOST_UNDO
            || (self.undo.len() > 1 && bytes(&self.undo) > MOST_UNDO_BYTES)
        {
            self.undo.remove(0);
        }
    }

    fn patch(&self, alpha: &[u8], b: Bounds) -> Patch {
        let (w, h) = (b.x1 - b.x0, b.y1 - b.y0);
        let rows = (b.y0..b.y1).flat_map(|y| {
            let at = y as usize * self.width as usize + b.x0 as usize;
            &alpha[at..at + w as usize]
        });
        Patch {
            x: b.x0,
            y: b.y0,
            w,
            h,
            alpha: rows.copied().collect(),
        }
    }

    /// Puts `patch` in, and gives what was there and where.
    fn swap(&mut self, patch: &Patch) -> (Patch, Bounds) {
        let bounds = Bounds {
            x0: patch.x,
            y0: patch.y,
            x1: patch.x + patch.w,
            y1: patch.y + patch.h,
        };
        let old = self.patch(&self.alpha, bounds);
        let w = patch.w as usize;
        for (row, line) in patch.alpha.chunks_exact(w).enumerate() {
            let at = (patch.y as usize + row) * self.width as usize + patch.x as usize;
            self.alpha[at..at + w].copy_from_slice(line);
        }
        (old, bounds)
    }

    /// Adds `alpha`, or takes it away, as a stroke that can be undone.
    pub fn put(&mut self, alpha: &[u8], adding: bool) {
        if alpha.len() != self.alpha.len() {
            return;
        }
        self.begin();
        for ((now, whole), put) in self.alpha.iter_mut().zip(&self.whole).zip(alpha) {
            *now = if adding {
                (*now).max(*put).min(*whole)
            } else {
                (*now).min(255 - put)
            };
        }
        self.touched = Some(Bounds {
            x0: 0,
            y0: 0,
            x1: self.width,
            y1: self.height,
        });
        self.end();
    }

    /// The stroke before goes; where.
    pub fn undo(&mut self) -> Option<Bounds> {
        let patch = self.undo.pop()?;
        let (old, bounds) = self.swap(&patch);
        self.redo.push(old);
        Some(bounds)
    }

    /// The stroke undone comes back; where.
    pub fn redo(&mut self) -> Option<Bounds> {
        let patch = self.redo.pop()?;
        let (old, bounds) = self.swap(&patch);
        self.undo.push(old);
        Some(bounds)
    }
}

/// How much of the brush covers what is `distance` from its middle.
fn cover(distance: f32, radius: f32) -> f32 {
    // A pixel of softness at least.
    let soft = (radius * (1.0 - HARD)).max(1.0);
    ((radius + 0.5 - distance) / soft).clamp(0.0, 1.0)
}

/// The longest side of the picture on the screen, in px.
const SHOWN: u32 = 4096;

/// The interface that erases the pixels of an image.
pub(crate) struct Eraser {
    pub page: PageId,
    pub node: NodeId,
    /// The image when it was opened.
    pub image: ImageNode,
    pub base: Pixels,
    pub paint: Paint,
    mode: Mode,
    size: f32,
    comparing: bool,
    /// How much smaller the picture is, and the pictures.
    step: u32,
    texture: Option<egui::TextureHandle>,
    whole: Option<egui::TextureHandle>,
    sight: Sight,
    /// The px of the image under the pointer.
    over: Option<(u32, u32)>,
    last: Option<(f32, f32)>,
}

/// Where the image is seen from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sight {
    /// How the scale was chosen, and the px of screen of a px of the image.
    view: Zoom,
    zoom: f32,
    /// The middle of the image, from the one of the room.
    offset: Vec2,
}

impl Default for Sight {
    fn default() -> Self {
        Self {
            view: Zoom::Fit,
            zoom: 1.0,
            offset: Vec2::ZERO,
        }
    }
}

/// The room left around an image that fits.
const AROUND: f32 = 64.0;

impl Sight {
    /// The scale an image of `size` fits in `room` at follows the room.
    fn settle(&mut self, size: Vec2, room: Vec2) {
        if self.view == Zoom::Fit {
            let room = (room - Vec2::splat(AROUND)).max(Vec2::splat(1.0));
            let fit = (room.x / size.x.max(1.0)).min(room.y / size.y.max(1.0));
            self.zoom = self.view.resolve(fit, fit);
            self.offset = Vec2::ZERO;
        }
    }

    /// The image goes along; it no longer fits by itself.
    fn drag(&mut self, by: Vec2) {
        self.view = Zoom::Fixed(self.zoom);
        self.offset += by;
    }

    /// What `ask` says, about `at` from the middle of the room: what is there
    /// stays there.
    fn ask(&mut self, ask: ZoomAsk, at: Vec2) {
        self.view = ask.apply(self.zoom);
        // «Ajustar» waits for the room.
        if let Zoom::Fixed(zoom) = self.view {
            self.offset = at + (self.offset - at) * (zoom / self.zoom);
            self.zoom = zoom;
        }
    }
}

/// What the person asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Done,
    Cancel,
}

impl Eraser {
    /// On the image `cut` (its opacity) of `base`; none when they are not
    /// alike.
    pub fn new(
        (page, node, image): (PageId, NodeId, ImageNode),
        base: Pixels,
        cut: &[u8],
    ) -> Option<Self> {
        let size = base.size();
        let whole = base.rgba().chunks_exact(4).map(|p| p[3]).collect();
        let paint = Paint::new(size, cut.to_vec(), whole)?;
        Some(Self {
            page,
            node,
            image,
            base,
            paint,
            mode: Mode::Erase,
            size: 40.0,
            comparing: false,
            step: size.0.max(size.1).div_ceil(SHOWN).max(1),
            texture: None,
            whole: None,
            sight: Sight::default(),
            over: None,
            last: None,
        })
    }

    /// The image with the opacity as it is painted.
    pub fn result(&self) -> Option<Pixels> {
        let (w, h) = self.base.size();
        let mut rgba = self.base.rgba().to_vec();
        for (pixel, alpha) in rgba.chunks_exact_mut(4).zip(self.paint.alpha()) {
            pixel[3] = *alpha;
        }
        Pixels::new(w, h, rgba)
    }

    /// The pixels of the picture in `bounds` of the image, and their place.
    fn picture(&self, bounds: Bounds, whole: bool) -> (egui::ColorImage, [usize; 2]) {
        let k = self.step;
        let (x0, y0) = (bounds.x0 / k, bounds.y0 / k);
        let (x1, y1) = (bounds.x1.div_ceil(k), bounds.y1.div_ceil(k));
        let (w, h) = self.base.size();
        let mut pixels = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
        for y in y0..y1 {
            for x in x0..x1 {
                let (x, y) = ((x * k).min(w - 1), (y * k).min(h - 1));
                let at = y as usize * w as usize + x as usize;
                let p = &self.base.rgba()[at * 4..at * 4 + 4];
                let alpha = if whole { p[3] } else { self.paint.alpha()[at] };
                pixels.push(Color32::from_rgba_unmultiplied(p[0], p[1], p[2], alpha));
            }
        }
        let size = [(x1 - x0) as usize, (y1 - y0) as usize];
        (
            egui::ColorImage::new(size, pixels),
            [x0 as usize, y0 as usize],
        )
    }

    fn all(&self) -> Bounds {
        let (x1, y1) = self.base.size();
        Bounds {
            x0: 0,
            y0: 0,
            x1,
            y1,
        }
    }

    /// The picture shows `bounds` as they are now.
    fn refresh(&mut self, bounds: Bounds) {
        let (image, at) = self.picture(bounds, false);
        if let Some(texture) = &mut self.texture {
            texture.set_partial(at, image, egui::TextureOptions::LINEAR);
        }
    }

    /// The interface, in place of the canvas and its panels.
    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) -> Option<Answer> {
        let mut answer = None;
        let (undo, redo, smaller, larger, escape) = ctx.input_mut(|i| {
            let mut key = |modifiers, key| i.consume_key(modifiers, key);
            let redo = key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            ) || key(egui::Modifiers::COMMAND, egui::Key::Y);
            (
                key(egui::Modifiers::COMMAND, egui::Key::Z),
                redo,
                key(egui::Modifiers::NONE, egui::Key::OpenBracket),
                key(egui::Modifiers::NONE, egui::Key::CloseBracket),
                key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        let mut changed = None;
        if undo {
            changed = self.paint.undo();
        }
        if redo {
            changed = self.paint.redo();
        }
        if smaller {
            self.size = (self.size / 1.2).clamp(*SIZES.start(), *SIZES.end());
        }
        if larger {
            self.size = (self.size * 1.2).clamp(*SIZES.start(), *SIZES.end());
        }
        if escape {
            answer = Some(Answer::Cancel);
        }
        // Ctrl with +, −, 0 and 1, as on the canvas; about the middle.
        let ask = ctx.input(|i| {
            use egui::Key;
            [Key::Equals, Key::Plus, Key::Minus, Key::Num0, Key::Num1]
                .into_iter()
                .filter(|key| i.key_pressed(*key))
                .find_map(|key| ZoomAsk::from_shortcut(i.modifiers.command, key))
        });
        let ask = self.status_bar(ctx, theme, texts).or(ask);
        if let Some(ask) = ask {
            self.sight.ask(ask, Vec2::ZERO);
        }
        egui::SidePanel::left("eraser.side")
            .exact_width(300.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                // Away from the edge, whatever the height of the window.
                let frame = egui::Frame::NONE.inner_margin(egui::Margin {
                    top: 12,
                    bottom: 12,
                    ..egui::Margin::ZERO
                });
                egui::TopBottomPanel::bottom("eraser.actions")
                    .show_separator_line(false)
                    .frame(frame)
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            if widgets::primary(ui, theme, plain(texts, "eraser.done")).clicked() {
                                answer = Some(Answer::Done);
                            }
                            let text = plain(texts, "eraser.cancel");
                            if widgets::secondary(ui, theme, text).clicked() {
                                answer = Some(Answer::Cancel);
                            }
                        });
                    });
                ui.label(RichText::new(plain(texts, "eraser.title")).font(theme.font("h3")));
                ui.add_space(14.0);
                ui.label(plain(texts, "eraser.kind"));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let half = (ui.available_width() - 8.0) / 2.0;
                    for (mode, key) in [
                        (Mode::Erase, "eraser.erase"),
                        (Mode::Restore, "eraser.restore"),
                    ] {
                        let on = self.mode == mode;
                        let button = egui::Button::selectable(on, plain(texts, key))
                            .min_size(Vec2::new(half, 36.0));
                        if ui.add(button).clicked() {
                            self.mode = mode;
                        }
                    }
                });
                ui.add_space(14.0);
                let mut size = f64::from(self.size);
                let range = (f64::from(*SIZES.start()), f64::from(*SIZES.end()));
                if crate::slider::Slider::new(plain(texts, "eraser.size"), &mut size, range)
                    .rest(range.0)
                    .logarithmic()
                    .step(1.0, 0)
                    .show(ui, theme)
                {
                    // Within SIZES: the cast loses nothing seen.
                    self.size = size as f32;
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.label(plain(texts, "eraser.compare"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        widgets::toggle(ui, theme, &mut self.comparing)
                            .on_hover_text(plain(texts, "eraser.compare_hint"));
                    });
                });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let button = |text: &'static str| egui::Button::new(plain(texts, text));
                    let can = self.paint.can_undo();
                    if ui.add_enabled(can, button("eraser.undo")).clicked() {
                        changed = self.paint.undo();
                    }
                    let can = self.paint.can_redo();
                    if ui.add_enabled(can, button("eraser.redo")).clicked() {
                        changed = self.paint.redo();
                    }
                });
                ui.add_space(8.0);
                let hint = RichText::new(plain(texts, "eraser.hint"))
                    .font(theme.font("caption"))
                    .color(theme.c("text.secondary"));
                ui.label(hint);
            });
        if let Some(bounds) = changed {
            self.refresh(bounds);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.c("workspace")))
            .show(ctx, |ui| self.view(ui, theme));
        answer
    }

    /// The bar along the bottom: the size of the image, the pixel under the
    /// pointer, and the zoom, as on the canvas. The zoom asked for.
    fn status_bar(&self, ctx: &egui::Context, theme: &Theme, texts: &Texts) -> Option<ZoomAsk> {
        egui::TopBottomPanel::bottom("eraser.status")
            .exact_height(px(theme.tokens.size.bottombar))
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::symmetric(12, 0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let label = |text: String| {
                        RichText::new(text)
                            .font(theme.font("ui-label"))
                            .color(theme.c("text.secondary"))
                    };
                    let (w, h) = self.base.size();
                    ui.label(label(texts.t(&gafira_doc::msg!(
                        "eraser.image_size",
                        width = Text(w.to_string()),
                        height = Text(h.to_string()),
                    ))));
                    if let Some((x, y)) = self.over {
                        bar_separator(ui, theme);
                        ui.label(label(format!("X {x}  Y {y}")));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let now = (self.sight.zoom, self.sight.view);
                        zoom_controls(ui, theme, texts, now)
                    })
                    .inner
                })
                .inner
            })
            .inner
    }

    fn view(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let options = egui::TextureOptions::LINEAR;
        if self.texture.is_none() {
            let (image, _) = self.picture(self.all(), false);
            self.texture = Some(ui.ctx().load_texture("eraser", image, options));
        }
        if self.comparing && self.whole.is_none() {
            let (image, _) = self.picture(self.all(), true);
            self.whole = Some(ui.ctx().load_texture("eraser-whole", image, options));
        }
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let (w, h) = self.base.size();
        // Sizes of images: the casts lose nothing seen.
        let size = Vec2::new(w as f32, h as f32);
        self.sight.settle(size, rect.size());
        self.scroll(ui, rect, &response);
        let place =
            Rect::from_center_size(rect.center() + self.sight.offset, size * self.sight.zoom);
        let painter = ui.painter_at(rect);
        checkers(&painter, place.intersect(rect));
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let shown = if self.comparing {
            &self.whole
        } else {
            &self.texture
        };
        if let Some(texture) = shown {
            painter.image(texture.id(), place, uv, Color32::WHITE);
        }
        let panning = ui.input(|i| i.pointer.middle_down() || i.key_down(egui::Key::Space));
        let Some(at) = response.hover_pos().or(response.interact_pointer_pos()) else {
            self.stop();
            self.over = None;
            return;
        };
        let to_image = |p: Pos2| {
            (
                (p.x - place.min.x) / self.sight.zoom,
                (p.y - place.min.y) / self.sight.zoom,
            )
        };
        let (x, y) = to_image(at);
        let on = x >= 0.0 && y >= 0.0 && x < size.x && y < size.y;
        // Inside the image: the casts drop the fraction alone.
        self.over = on.then_some((x as u32, y as u32));
        if panning {
            self.stop();
            self.sight.drag(response.drag_delta());
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            return;
        }
        let down = ui.input(|i| i.pointer.primary_down()) && response.is_pointer_button_down_on();
        if down && !self.comparing {
            let to = to_image(at);
            let from = match self.last {
                Some(last) => last,
                None => {
                    self.paint.begin();
                    to
                }
            };
            self.last = Some(to);
            if let Some(bounds) = self.paint.line(self.mode, from, to, self.size) {
                self.refresh(bounds);
            }
        } else {
            self.stop();
        }
        // The brush: a light ring in a dark one, seen on any image.
        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
        let radius = (self.size / 2.0 * self.sight.zoom).max(1.5);
        painter.circle_stroke(at, radius + 1.0, Stroke::new(1.0, Color32::BLACK));
        let color = match self.mode {
            Mode::Erase => Color32::WHITE,
            Mode::Restore => theme.c("accent"),
        };
        painter.circle_stroke(at, radius, Stroke::new(1.5, color));
    }

    /// The stroke is over, if there is one.
    fn stop(&mut self) {
        if self.last.take().is_some() {
            self.paint.end();
        }
    }

    /// Ctrl and the wheel zoom on the pointer; the wheel moves the image.
    fn scroll(&mut self, ui: &egui::Ui, rect: Rect, response: &egui::Response) {
        let Some(at) = response.hover_pos() else {
            return;
        };
        let (factor, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
        if factor != 1.0 {
            let to = zoom::by_factor(self.sight.zoom, factor);
            self.sight.ask(ZoomAsk::To(to), at - rect.center());
        } else if scroll != Vec2::ZERO {
            self.sight.drag(scroll);
        }
    }
}

/// The squares behind what is see-through.
fn checkers(painter: &egui::Painter, rect: Rect) {
    const SIDE: f32 = 10.0;
    if !rect.is_positive() {
        return;
    }
    painter.rect_filled(rect, 0.0, Color32::WHITE);
    let painter = painter.with_clip_rect(rect);
    let (columns, rows) = (
        (rect.width() / SIDE).ceil() as u32,
        (rect.height() / SIDE).ceil() as u32,
    );
    let mut mesh = egui::Mesh::default();
    for row in 0..rows {
        for col in (row % 2..columns).step_by(2) {
            let at = rect.min + Vec2::new(col as f32 * SIDE, row as f32 * SIDE);
            let square = Rect::from_min_size(at, Vec2::splat(SIDE));
            mesh.add_colored_rect(square, Color32::from_gray(214));
        }
    }
    painter.add(mesh);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint(w: u32, h: u32) -> Paint {
        let count = (w * h) as usize;
        Paint::new((w, h), vec![255; count], vec![255; count]).expect("a paint")
    }

    fn at(p: &Paint, x: u32, y: u32) -> u8 {
        p.alpha()[(y * p.width + x) as usize]
    }

    // S84-08, FD-39 (b): erasing leaves a hole of its size, and bringing
    // back is its inverse.
    #[test]
    fn s84_08_erase_and_restore() {
        let mut p = paint(100, 60);
        p.begin();
        p.line(Mode::Erase, (20.0, 30.0), (70.0, 30.0), 20.0);
        p.end();
        assert_eq!((at(&p, 45, 30), at(&p, 20, 30), at(&p, 45, 35)), (0, 0, 0));
        assert_eq!(
            (at(&p, 45, 50), at(&p, 95, 30), at(&p, 5, 30)),
            (255, 255, 255)
        );
        p.begin();
        p.line(Mode::Restore, (20.0, 30.0), (70.0, 30.0), 30.0);
        p.end();
        assert!(p.alpha().iter().all(|a| *a == 255));
    }

    // S84-01: what the image never had does not appear.
    #[test]
    fn s84_08_restoring_stays_within_the_image() {
        let whole: Vec<u8> = (0..100).map(|i| if i < 50 { 0 } else { 90 }).collect();
        let mut p = Paint::new((10, 10), vec![0; 100], whole.clone()).expect("a paint");
        p.begin();
        p.line(Mode::Restore, (5.0, 5.0), (5.0, 5.0), 60.0);
        p.end();
        assert_eq!(p.alpha(), whole);
        assert!(Paint::new((10, 10), vec![0; 99], vec![0; 100]).is_none());
        assert!(Paint::new((0, 0), vec![], vec![]).is_none());
    }

    // S84-08: a step for each stroke, back and forth; one that changes
    // nothing is none.
    #[test]
    fn s84_08_undo_and_redo() {
        let mut p = paint(40, 40);
        let clean = p.alpha().to_vec();
        for x in [10.0, 30.0] {
            p.begin();
            p.line(Mode::Erase, (x, 10.0), (x, 20.0), 6.0);
            p.line(Mode::Erase, (x, 20.0), (x, 30.0), 6.0);
            p.end();
        }
        let both = p.alpha().to_vec();
        assert!(p.undo().is_some());
        assert_eq!((at(&p, 10, 15), at(&p, 30, 15)), (0, 255));
        assert!(p.undo().is_some() && p.undo().is_none());
        assert_eq!(p.alpha(), clean);
        assert!(p.redo().is_some() && p.redo().is_some() && p.redo().is_none());
        assert_eq!(p.alpha(), both);
        p.begin();
        p.line(Mode::Restore, (2.0, 2.0), (3.0, 2.0), 2.0);
        p.end();
        assert!(!p.can_redo() && p.undo.len() == 2, "nothing changed");
        p.undo();
        p.begin();
        p.line(Mode::Erase, (20.0, 20.0), (20.0, 20.0), 4.0);
        p.end();
        assert!(!p.can_redo(), "another way");
    }

    // S84-08: it fits until a scale is asked for; then the point zoomed at
    // stays, within the scales of the canvas.
    #[test]
    fn s84_08_the_zoom() {
        let (size, room) = (Vec2::new(1000.0, 500.0), Vec2::new(564.0, 464.0));
        let mut s = Sight::default();
        s.settle(size, room);
        assert_eq!((s.zoom, s.offset), (0.5, Vec2::ZERO));
        s.ask(ZoomAsk::In, Vec2::ZERO);
        assert_eq!((s.zoom, s.view), (0.67, Zoom::Fixed(0.67)));
        s.settle(size, room * 2.0);
        assert_eq!(s.zoom, 0.67, "no longer follows the room");
        // The px of the image under the pointer is the same one after.
        let at = Vec2::new(90.0, -40.0);
        let under = |s: &Sight| (at - s.offset) / s.zoom;
        let before = under(&s);
        for ask in [
            ZoomAsk::To(3.0),
            ZoomAsk::Out,
            ZoomAsk::In,
            ZoomAsk::To(0.4),
        ] {
            s.ask(ask, at);
            assert!((under(&s) - before).length() < 1e-2, "{ask:?}");
        }
        s.ask(ZoomAsk::To(900.0), at);
        assert_eq!(s.zoom, zoom::MAX);
        s.ask(ZoomAsk::To(0.0), at);
        assert_eq!(s.zoom, zoom::MIN);
        s.ask(ZoomAsk::Fit, at);
        s.settle(size, room);
        assert_eq!((s.zoom, s.offset), (0.5, Vec2::ZERO));
        s.settle(size, Vec2::ZERO);
        assert!(s.zoom >= zoom::MIN && s.zoom.is_finite());
    }

    // S84-08: outside the image, or with odd numbers, nothing breaks.
    #[test]
    fn s84_08_outside_the_image() {
        let mut p = paint(20, 20);
        p.begin();
        for (a, b) in [
            ((-50.0, -50.0), (-40.0, -40.0)),
            ((500.0, 5.0), (600.0, 5.0)),
            ((f32::NAN, 5.0), (6.0, 5.0)),
        ] {
            assert_eq!(p.line(Mode::Erase, a, b, 10.0), None);
        }
        p.end();
        assert!(!p.can_undo());
        p.begin();
        assert!(
            p.line(Mode::Erase, (-100.0, 10.0), (200.0, 10.0), 500.0)
                .is_some()
        );
        p.end();
        assert!(p.alpha().iter().all(|a| *a == 0));
    }
}
