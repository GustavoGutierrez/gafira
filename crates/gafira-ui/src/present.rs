//! Presentar en pantalla completa (spec 101): the pages of the design one at a time on the
//! whole screen, as large as they fit on a dark ground. The arrow keys, Space and a click
//! move between them; the bar at the bottom, with the page number, the arrows and the way
//! out, shows while the pointer is near the bottom edge and hides when it leaves.

use std::collections::BTreeMap;

use egui::{Align2, Color32, Key, Pos2, Rect, Stroke, Vec2};
use gafira_doc::PageId;

use crate::charts::plain;
use crate::icons::{self, Icon, Weight};
use crate::style::Theme;
use crate::text::Texts;

/// How near the bottom edge, in points, the pointer shows the bar.
const REVEAL: f32 = 96.0;
/// How long the bar stays when the presentation starts, in seconds.
const INTRO: f64 = 2.5;
const BAR: f32 = 56.0;
/// The slide fades in this long once the screen has its full size, in seconds.
const FADE_IN: f64 = 0.35;
/// A change of page cross-fades and slides this long, in seconds.
const TURN: f64 = 0.42;
/// How far a page slides in a change, in points.
const SLIDE: f32 = 56.0;
/// How long the screen size must hold still before the slide shows, in seconds.
const SETTLE: f64 = 0.12;
/// The slide shows after this long even if the window never reports full screen.
const SETTLE_MOST: f64 = 1.2;

/// Exponential-feeling ease out: fast start, soft landing.
fn ease_out(t: f64) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (1.0 - (1.0 - t).powi(3)) as f32
}

/// The presentation tools (S101-08): a laser pointer and drawing over the slide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tool {
    #[default]
    None,
    Laser,
    Pen,
    Marker,
    Highlighter,
    Eraser,
}

impl Tool {
    fn draws(self) -> bool {
        matches!(self, Self::Pen | Self::Marker | Self::Highlighter)
    }

    fn drawing(self) -> bool {
        self.draws() || self == Self::Eraser
    }

    /// Width as a share of the slide's height, and opacity.
    fn stroke(self) -> (f32, f32) {
        match self {
            Self::Marker => (0.009, 1.0),
            Self::Highlighter => (0.028, 0.42),
            _ => (0.0035, 1.0),
        }
    }
}

/// The colours of the drawing tools, as theme tokens.
const INKS: [&str; 6] = [
    "gizmo.x",
    "primary.bg",
    "present.marker",
    "gizmo.y",
    "gizmo.z",
    "topbar.bg",
];

/// A stroke drawn over a slide, in the slide's own 0–1 coordinates, so it stays on its
/// page whatever the screen. Drawings are not saved in the design.
#[derive(Clone, Debug)]
struct Mark {
    points: Vec<Pos2>,
    ink: usize,
    tool: Tool,
}

/// How long the laser leaves a trail, in seconds.
const TRAIL: f64 = 0.35;

/// Draws a page to fit a pixel size: its width, height and straight-alpha RGBA.
pub(crate) type PageDrawer<'a> = dyn FnMut(PageId, (u32, u32)) -> Option<(u32, u32, Vec<u8>)> + 'a;

/// What the presentation asks of the editor.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Asks {
    /// Leave the presentation.
    pub(crate) exit: bool,
}

/// An open presentation: the pages in order, the one shown, and their pictures by size.
pub(crate) struct Presenting {
    pub(crate) pages: Vec<PageId>,
    pub(crate) index: usize,
    started: Option<f64>,
    pictures: BTreeMap<(PageId, u32, u32), egui::TextureHandle>,
    /// The window was not full screen before: put it back on leaving.
    pub(crate) was_windowed: bool,
    /// The window was already asked to go full screen.
    pub(crate) opened: bool,
    /// When the presentation opened, and the last screen size and since when it holds.
    opened_at: Option<f64>,
    settling: Option<(Vec2, f64)>,
    /// When the screen reached its full size: the slide fades in from then.
    ready_at: Option<f64>,
    /// The page on the screen, and the change under way: the page it leaves, when it
    /// started and its direction.
    shown: usize,
    turning: Option<(usize, f64, f32)>,
    /// The tool in hand, the ink, the drawings of each page, the stroke under way and the
    /// laser's recent positions.
    pub(crate) tool: Tool,
    ink: usize,
    marks: BTreeMap<usize, Vec<Mark>>,
    stroke: Option<Mark>,
    trail: Vec<(Pos2, f64)>,
}

impl Presenting {
    /// A presentation of `pages` from `start` (the current page when it is among them).
    pub(crate) fn new(pages: Vec<PageId>, start: Option<PageId>, was_windowed: bool) -> Self {
        let index = start
            .and_then(|s| pages.iter().position(|p| *p == s))
            .unwrap_or(0);
        Self {
            pages,
            index,
            started: None,
            pictures: BTreeMap::new(),
            was_windowed,
            opened: false,
            opened_at: None,
            settling: None,
            ready_at: None,
            shown: index,
            turning: None,
            tool: Tool::None,
            ink: 0,
            marks: BTreeMap::new(),
            stroke: None,
            trail: Vec::new(),
        }
    }

    /// Moves `by` pages, staying within the first and the last.
    pub(crate) fn go(&mut self, by: isize) {
        let last = self.pages.len().saturating_sub(1) as isize;
        self.index = (self.index as isize + by).clamp(0, last) as usize;
    }

    pub(crate) fn current(&self) -> Option<PageId> {
        self.pages.get(self.index).copied()
    }

    /// The keys of the presentation (S101-04): next with Right, Down, Space, Page Down and
    /// Enter; back with Left, Up, Page Up and Backspace; Home and End; Esc leaves.
    pub(crate) fn keys(&mut self, ctx: &egui::Context) -> Asks {
        let mut asks = Asks::default();
        ctx.input(|i| {
            for key in [
                Key::ArrowRight,
                Key::ArrowDown,
                Key::Space,
                Key::PageDown,
                Key::Enter,
            ] {
                if i.key_pressed(key) {
                    self.go(1);
                }
            }
            for key in [Key::ArrowLeft, Key::ArrowUp, Key::PageUp, Key::Backspace] {
                if i.key_pressed(key) {
                    self.go(-1);
                }
            }
            if i.key_pressed(Key::Home) {
                self.index = 0;
            }
            if i.key_pressed(Key::End) {
                self.index = self.pages.len().saturating_sub(1);
            }
            // Tools: L laser, D draw, E eraser, C clears the page's drawings.
            let toggle = |now: Tool, to: Tool| if now == to { Tool::None } else { to };
            if i.key_pressed(Key::L) {
                self.tool = toggle(self.tool, Tool::Laser);
            }
            if i.key_pressed(Key::D) {
                self.tool = if self.tool.draws() {
                    Tool::None
                } else {
                    Tool::Pen
                };
            }
            if i.key_pressed(Key::E) {
                self.tool = toggle(self.tool, Tool::Eraser);
            }
            if i.key_pressed(Key::C) {
                self.marks.remove(&self.index);
            }
            // Esc puts the tool down first, then leaves.
            if i.key_pressed(Key::Escape) {
                if self.tool == Tool::None {
                    asks.exit = true;
                } else {
                    self.tool = Tool::None;
                }
            }
        });
        asks
    }

    /// Removes the drawings of the current page that pass within `reach` of `at`
    /// (slide coordinates).
    fn erase(&mut self, at: Pos2, reach: Vec2) {
        if let Some(marks) = self.marks.get_mut(&self.index) {
            marks.retain(|m| {
                !m.points
                    .iter()
                    .any(|p| ((p.x - at.x) / reach.x).hypot((p.y - at.y) / reach.y) < 1.0)
            });
        }
    }

    /// Draws the presentation over the whole window. `draw` renders a page to fit a pixel
    /// size, as straight-alpha RGBA.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        draw: &mut PageDrawer<'_>,
    ) -> Asks {
        let mut asks = self.keys(ctx);
        let now = ctx.input(|i| i.time);
        let screen = ctx.content_rect();
        let ppp = ctx.pixels_per_point();

        // While the window grows to the whole screen only the dark ground shows, so the
        // slide never jumps from the window's size to the screen's (S101-06).
        let opened_at = *self.opened_at.get_or_insert(now);
        if self.ready_at.is_none() {
            let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            let held = match self.settling {
                Some((size, since)) if size == screen.size() => now - since,
                _ => {
                    self.settling = Some((screen.size(), now));
                    0.0
                }
            };
            if (full && held >= SETTLE) || now - opened_at >= SETTLE_MOST {
                self.ready_at = Some(now);
                self.started = Some(now);
            }
        }
        let Some(ready_at) = self.ready_at else {
            egui::Area::new(egui::Id::new("present.stage"))
                .order(egui::Order::Foreground)
                .fixed_pos(screen.min)
                .show(ctx, |ui| {
                    let (rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, theme.c("present.bg"));
                });
            ctx.request_repaint();
            return asks;
        };
        let started = self.started.unwrap_or(ready_at);
        let fit = (
            (screen.width() * ppp) as u32,
            (screen.height() * ppp) as u32,
        );

        // The page shown, then its neighbours, one new picture a frame.
        let order = [
            Some(self.index),
            self.index.checked_add(1),
            self.index.checked_sub(1),
        ];
        let mut drew = false;
        for index in order.into_iter().flatten() {
            let Some(page) = self.pages.get(index).copied() else {
                continue;
            };
            if self.pictures.contains_key(&(page, fit.0, fit.1)) {
                continue;
            }
            if drew {
                ctx.request_repaint();
                break;
            }
            if let Some((w, h, rgba)) = draw(page, fit) {
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                let texture = ctx.load_texture(
                    format!("present-{page:?}"),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                self.pictures.insert((page, fit.0, fit.1), texture);
            }
            drew = true;
        }
        // Keep the pictures of this screen size only.
        self.pictures.retain(|(_, w, h), _| (*w, *h) == fit);

        // A change of page starts its transition (S101-07).
        if self.index != self.shown {
            let dir = if self.index > self.shown { 1.0 } else { -1.0 };
            self.turning = Some((self.shown, now, dir));
            self.shown = self.index;
        }
        if self
            .turning
            .is_some_and(|(_, since, _)| now - since >= TURN)
        {
            self.turning = None;
        }
        let fade = ease_out((now - ready_at) / FADE_IN);
        if fade < 1.0 || self.turning.is_some() {
            ctx.request_repaint();
        }

        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let near_bottom = pointer.is_some_and(|p| p.y > screen.bottom() - REVEAL);
        let bar_shown = near_bottom || now - started < INTRO;
        if !bar_shown {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }

        egui::Area::new(egui::Id::new("present.stage"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
                let painter = ui.painter();
                painter.rect_filled(rect, 0.0, theme.c("present.bg"));
                let place = |texture: &egui::TextureHandle, shift: f32| {
                    let size = texture.size_vec2() / ppp;
                    let k = (rect.width() / size.x).min(rect.height() / size.y);
                    Rect::from_center_size(rect.center() + Vec2::new(shift, 0.0), size * k)
                };
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                let picture_of = |index: usize| {
                    self.pages
                        .get(index)
                        .and_then(|p| self.pictures.get(&(*p, fit.0, fit.1)))
                };
                let current = picture_of(self.index);
                match (self.turning, current) {
                    (Some((from, since, dir)), Some(texture)) => {
                        let t = ease_out((now - since) / TURN);
                        if let Some(old) = picture_of(from) {
                            let tint = Color32::WHITE.gamma_multiply(1.0 - t);
                            painter.image(old.id(), place(old, -dir * SLIDE * t), uv, tint);
                        }
                        let tint = Color32::WHITE.gamma_multiply(t * fade);
                        painter.image(
                            texture.id(),
                            place(texture, dir * SLIDE * (1.0 - t)),
                            uv,
                            tint,
                        );
                    }
                    (_, Some(texture)) => {
                        let tint = Color32::WHITE.gamma_multiply(fade);
                        painter.image(texture.id(), place(texture, 0.0), uv, tint);
                    }
                    (_, None) => ctx.request_repaint(),
                }
                // Where the slide is on the screen, for the drawings (S101-08).
                let slide = current.map(|t| place(t, 0.0));
                let on_bar = bar_shown && pointer.is_some_and(|p| p.y > rect.bottom() - BAR - 60.0);
                if let Some(slide) = slide {
                    let to_uv = |p: Pos2| {
                        Pos2::new(
                            (p.x - slide.left()) / slide.width(),
                            (p.y - slide.top()) / slide.height(),
                        )
                    };
                    let to_screen =
                        |p: Pos2| slide.min + Vec2::new(p.x * slide.width(), p.y * slide.height());
                    let tool = self.tool;
                    if tool.draws() && !on_bar {
                        if response.drag_started()
                            && let Some(p) = pointer
                        {
                            self.stroke = Some(Mark {
                                points: vec![to_uv(p)],
                                ink: self.ink,
                                tool,
                            });
                        }
                        if response.dragged()
                            && let (Some(stroke), Some(p)) = (self.stroke.as_mut(), pointer)
                        {
                            let uv = to_uv(p);
                            if stroke
                                .points
                                .last()
                                .is_none_or(|l| (*l - uv).length() > 0.0015)
                            {
                                stroke.points.push(uv);
                            }
                        }
                        if response.drag_stopped()
                            && let Some(stroke) = self.stroke.take()
                        {
                            self.marks.entry(self.index).or_default().push(stroke);
                        }
                    }
                    if tool == Tool::Eraser
                        && !on_bar
                        && (response.dragged() || response.clicked())
                        && let Some(p) = pointer
                    {
                        let reach = Vec2::new(14.0 / slide.width(), 14.0 / slide.height());
                        self.erase(to_uv(p), reach);
                    }
                    // The drawings of this page, while no change of page is under way.
                    if self.turning.is_none() {
                        let all = self
                            .marks
                            .get(&self.index)
                            .into_iter()
                            .flatten()
                            .chain(self.stroke.as_ref());
                        for mark in all {
                            let (width, alpha) = mark.tool.stroke();
                            let color = theme
                                .c(INKS[mark.ink % INKS.len()])
                                .gamma_multiply(alpha * fade);
                            let points: Vec<Pos2> =
                                mark.points.iter().map(|p| to_screen(*p)).collect();
                            let w = (width * slide.height()).max(1.5);
                            if points.len() == 1 {
                                painter.circle_filled(points[0], w / 2.0, color);
                            } else {
                                painter.add(egui::Shape::line(points, Stroke::new(w, color)));
                            }
                        }
                    }
                }
                // The laser: a bright dot with a short fading trail.
                if self.tool == Tool::Laser && !on_bar {
                    if let Some(p) = pointer
                        && self.trail.last().is_none_or(|(l, _)| *l != p)
                    {
                        self.trail.push((p, now));
                    }
                    self.trail.retain(|(_, t)| now - *t < TRAIL);
                    let red = theme.c("gizmo.x");
                    for pair in self.trail.windows(2) {
                        let age = ((now - pair[1].1) / TRAIL) as f32;
                        let w = 7.0 * (1.0 - age);
                        painter.line_segment(
                            [pair[0].0, pair[1].0],
                            Stroke::new(w, red.gamma_multiply(0.6 * (1.0 - age))),
                        );
                    }
                    if let Some(p) = pointer {
                        painter.circle_filled(p, 16.0, red.gamma_multiply(0.18));
                        painter.circle_filled(p, 10.0, red.gamma_multiply(0.35));
                        painter.circle_filled(p, 6.0, red);
                        painter.circle_filled(p, 2.5, Color32::WHITE.gamma_multiply(0.85));
                    }
                    ctx.request_repaint();
                } else {
                    self.trail.clear();
                }
                // A click on the slide goes to the next one, with no tool in hand.
                if response.clicked() && !on_bar && self.tool == Tool::None {
                    self.go(1);
                }
                let cursor = match self.tool {
                    _ if on_bar => None,
                    Tool::Laser => Some(egui::CursorIcon::None),
                    t if t.drawing() => Some(egui::CursorIcon::Crosshair),
                    _ if !bar_shown => Some(egui::CursorIcon::None),
                    _ => None,
                };
                if let Some(cursor) = cursor {
                    ctx.set_cursor_icon(cursor);
                }
            });

        if bar_shown {
            asks.exit |= self.bar(ctx, theme, texts, screen);
        }
        asks
    }

    /// The bar at the bottom: the progress, back, «n/N», next, and the way out.
    fn bar(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts, screen: Rect) -> bool {
        let mut exit = false;
        let rect = Rect::from_min_max(Pos2::new(screen.left(), screen.bottom() - BAR), screen.max);
        egui::Area::new(egui::Id::new("present.bar"))
            .order(egui::Order::Tooltip)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                let painter = ui.painter();
                painter.rect_filled(rect, 0.0, theme.c("topbar.bg").gamma_multiply(0.92));
                // How far through the design: a line along the top of the bar.
                let total = self.pages.len().max(1) as f32;
                let done = (self.index + 1) as f32 / total;
                let line = Rect::from_min_size(rect.min, Vec2::new(rect.width() * done, 3.0));
                painter.rect_filled(line, 0.0, theme.c("accent"));

                let fg = theme.c("topbar.fg");
                let button =
                    |ui: &mut egui::Ui, at: Rect, icon: Icon, name: &'static str, on: bool| {
                        let response = ui.interact(at, ui.id().with(name), egui::Sense::click());
                        let color = if on { fg } else { fg.gamma_multiply(0.35) };
                        if response.hovered() && on {
                            ui.painter().rect_filled(at, 8.0, theme.c("topbar.hover"));
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        icons::paint(
                            ui.painter(),
                            Rect::from_center_size(at.center(), Vec2::splat(22.0)),
                            icon,
                            Weight::Regular,
                            color,
                        );
                        response.on_hover_text(plain(texts, name)).clicked() && on
                    };
                let y = rect.center().y + 1.5;
                let left =
                    Rect::from_center_size(Pos2::new(rect.left() + 36.0, y), Vec2::splat(40.0));
                if button(
                    ui,
                    left,
                    Icon::CaretLeft,
                    "present.previous",
                    self.index > 0,
                ) {
                    self.go(-1);
                }
                ui.painter().text(
                    Pos2::new(rect.left() + 80.0, y),
                    Align2::CENTER_CENTER,
                    format!("{}/{}", self.index + 1, self.pages.len()),
                    theme.font("ui-label"),
                    fg,
                );
                let right =
                    Rect::from_center_size(Pos2::new(rect.left() + 124.0, y), Vec2::splat(40.0));
                if button(
                    ui,
                    right,
                    Icon::Submenu,
                    "present.next",
                    self.index + 1 < self.pages.len(),
                ) {
                    self.go(1);
                }
                let out =
                    Rect::from_center_size(Pos2::new(rect.right() - 36.0, y), Vec2::splat(40.0));
                ui.painter().line_segment(
                    [
                        Pos2::new(out.left() - 12.0, rect.top() + 10.0),
                        Pos2::new(out.left() - 12.0, rect.bottom() - 10.0),
                    ],
                    Stroke::new(1.0, fg.gamma_multiply(0.25)),
                );
                if button(ui, out, Icon::CornersIn, "present.exit", true) {
                    exit = true;
                }
                // The tools, left of the way out (S101-08).
                let toggle =
                    |ui: &mut egui::Ui, at: Rect, icon: Icon, name: &'static str, on: bool| {
                        if on {
                            ui.painter().rect_filled(at, 8.0, theme.c("primary.bg"));
                        }
                        let response = ui.interact(at, ui.id().with(name), egui::Sense::click());
                        if response.hovered() {
                            if !on {
                                ui.painter().rect_filled(at, 8.0, theme.c("topbar.hover"));
                            }
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        let color = if on { theme.c("primary.fg") } else { fg };
                        icons::paint(
                            ui.painter(),
                            Rect::from_center_size(at.center(), Vec2::splat(22.0)),
                            icon,
                            Weight::Regular,
                            color,
                        );
                        response.on_hover_text(plain(texts, name)).clicked()
                    };
                let slot = |n: f32| {
                    Rect::from_center_size(
                        Pos2::new(out.left() - 36.0 - n * 46.0, y),
                        Vec2::splat(40.0),
                    )
                };
                if toggle(
                    ui,
                    slot(0.0),
                    Icon::PenTool,
                    "present.draw",
                    self.tool.drawing(),
                ) {
                    self.tool = if self.tool.drawing() {
                        Tool::None
                    } else {
                        Tool::Pen
                    };
                }
                if toggle(
                    ui,
                    slot(1.0),
                    Icon::Laser,
                    "present.laser",
                    self.tool == Tool::Laser,
                ) {
                    self.tool = if self.tool == Tool::Laser {
                        Tool::None
                    } else {
                        Tool::Laser
                    };
                }
                let has_marks = self.marks.get(&self.index).is_some_and(|m| !m.is_empty());
                if has_marks && toggle(ui, slot(2.0), Icon::ClearMarks, "present.clear", false) {
                    self.marks.remove(&self.index);
                }
                // The drawing tools and inks, over the draw button.
                if self.tool.drawing() {
                    let anchor = slot(0.0);
                    let width = 4.0 * 44.0 + 6.0 * 30.0 + 40.0;
                    let at = Pos2::new(
                        (anchor.right() - width).max(screen.left() + 8.0),
                        rect.top() - 64.0,
                    );
                    egui::Area::new(egui::Id::new("present.tools"))
                        .order(egui::Order::Tooltip)
                        .fixed_pos(at)
                        .show(ui.ctx(), |ui| {
                            egui::Frame::NONE
                                .fill(theme.c("topbar.bg"))
                                .corner_radius(12)
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        for (tool, icon, name) in [
                                            (Tool::Pen, Icon::PenTool, "present.pen"),
                                            (Tool::Marker, Icon::MarkerTool, "present.marker"),
                                            (
                                                Tool::Highlighter,
                                                Icon::Highlighter,
                                                "present.highlighter",
                                            ),
                                            (Tool::Eraser, Icon::EraserTool, "present.eraser"),
                                        ] {
                                            let (r, _) = ui.allocate_exact_size(
                                                Vec2::splat(40.0),
                                                egui::Sense::hover(),
                                            );
                                            if toggle(ui, r, icon, name, self.tool == tool) {
                                                self.tool = tool;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        for (i, token) in INKS.iter().enumerate() {
                                            let (r, response) = ui.allocate_exact_size(
                                                Vec2::splat(26.0),
                                                egui::Sense::click(),
                                            );
                                            let on = self.ink == i;
                                            ui.painter().circle_filled(
                                                r.center(),
                                                if on { 11.0 } else { 9.0 },
                                                theme.c(token),
                                            );
                                            if on {
                                                ui.painter().circle_stroke(
                                                    r.center(),
                                                    13.0,
                                                    Stroke::new(2.0, fg),
                                                );
                                            } else if *token == "topbar.bg" {
                                                ui.painter().circle_stroke(
                                                    r.center(),
                                                    9.0,
                                                    Stroke::new(1.0, fg.gamma_multiply(0.5)),
                                                );
                                            }
                                            if response.hovered() {
                                                ui.ctx().set_cursor_icon(
                                                    egui::CursorIcon::PointingHand,
                                                );
                                            }
                                            if response
                                                .on_hover_text(plain(texts, "present.color"))
                                                .clicked()
                                            {
                                                self.ink = i;
                                                if self.tool == Tool::Eraser {
                                                    self.tool = Tool::Pen;
                                                }
                                            }
                                        }
                                    });
                                });
                        });
                }
            });
        exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(n: u64) -> Vec<PageId> {
        (1..=n).map(PageId::new).collect()
    }

    // S101-03: it starts on the current page and moves page by page within the design.
    #[test]
    fn s101_03_it_moves_within_the_pages() {
        let mut p = Presenting::new(pages(5), Some(PageId::new(3)), true);
        assert_eq!(p.index, 2);
        p.go(1);
        p.go(1);
        p.go(1);
        assert_eq!(p.index, 4, "stays on the last page");
        p.go(-10);
        assert_eq!(p.index, 0, "stays on the first page");
        let q = Presenting::new(pages(2), Some(PageId::new(9)), false);
        assert_eq!(q.index, 0, "an unknown page starts at the first");
        let mut drawn = Presenting::new(pages(2), None, false);
        drawn.marks.insert(
            0,
            vec![Mark {
                points: vec![Pos2::new(0.5, 0.5)],
                ink: 0,
                tool: Tool::Pen,
            }],
        );
        drawn.erase(Pos2::new(0.9, 0.9), Vec2::splat(0.05));
        assert_eq!(
            drawn.marks[&0].len(),
            1,
            "far from the stroke nothing is erased"
        );
        drawn.erase(Pos2::new(0.51, 0.5), Vec2::splat(0.05));
        assert!(drawn.marks[&0].is_empty(), "near it the stroke goes");
        let empty = Presenting::new(Vec::new(), None, false);
        assert_eq!(empty.current(), None);
    }
}
