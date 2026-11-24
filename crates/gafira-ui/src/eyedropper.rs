//! «Tomar un color» (spec 098): an eyedropper beside every colour picker of
//! the editor. Pressing it arms the dropper; the next click anywhere in the
//! window (the canvas, an image, a swatch) takes the colour of the pixel
//! under the pointer and hands it to the picker that armed it. Esc or a
//! right click leaves without a colour.
//!
//! The pixel is read from a screenshot of the window, so what is taken is
//! exactly what is seen: the rendered canvas, gradients, effects and images
//! included. Nothing here touches the document; the picker that receives the
//! colour applies it as if it had been chosen by hand.

use egui::{Color32, ColorImage, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::icons::{self, Icon, Weight};

/// The state of the dropper, kept in egui's memory for the whole window.
#[derive(Clone, Debug, Default, PartialEq)]
struct State {
    /// The picker waiting for a click on the window.
    armed: Option<Id>,
    /// Frames to wait after arming before the screenshot, so a picker that
    /// was open has closed and is not in it.
    settle: u8,
    /// The screenshot asked for and not arrived yet.
    requested: bool,
    /// The window as it was when the dropper was armed: the loupe shows it
    /// and the click reads it.
    shot: Option<std::sync::Arc<ColorImage>>,
    /// The picker and the point clicked, waiting for the screenshot.
    waiting: Option<(Id, Pos2)>,
    /// The colour taken, for the picker that asked for it.
    picked: Option<(Id, [u8; 3])>,
    /// The accessible name and tooltip of the button, from the catalog.
    name: String,
    /// The colour of the icon and of its armed background.
    ink: Option<(Color32, Color32)>,
}

fn state_id() -> Id {
    Id::new("eyedropper.state")
}

fn read(ctx: &egui::Context) -> State {
    ctx.data(|d| d.get_temp::<State>(state_id()))
        .unwrap_or_default()
}

fn write(ctx: &egui::Context, state: State) {
    ctx.data_mut(|d| d.insert_temp(state_id(), state));
}

/// The colour of the pixel at `at` (points) of a screenshot taken at `ppp`
/// pixels per point; none outside the image. The screenshot is opaque, so
/// only red, green and blue are kept.
pub(crate) fn pixel_at(image: &ColorImage, at: Pos2, ppp: f32) -> Option<[u8; 3]> {
    let (x, y) = (at.x * ppp, at.y * ppp);
    if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
        return None;
    }
    // Non-negative and finite: the casts only drop the fraction.
    let (x, y) = (x.floor() as usize, y.floor() as usize);
    let [w, h] = image.size;
    if x >= w || y >= h {
        return None;
    }
    let pixel = image.pixels[y * w + x];
    Some([pixel.r(), pixel.g(), pixel.b()])
}

/// `taken` put into `rgba`, keeping its transparency; a colour that could not
/// be seen (fully transparent) becomes opaque, or the pick would not show.
pub(crate) fn with_taken(rgba: [u8; 4], taken: [u8; 3]) -> [u8; 4] {
    let alpha = if rgba[3] == 0 { 255 } else { rgba[3] };
    [taken[0], taken[1], taken[2], alpha]
}

/// Once per frame, before the screens draw: keep the button's name and
/// colours and read the screenshot that arrived. While the dropper is armed,
/// a screenshot of the window is taken once, a loupe shows its pixels under
/// the pointer, and the window is covered so the click goes to the dropper
/// and not to the canvas; the click takes the pixel at once.
pub(crate) fn frame(ctx: &egui::Context, name: String, ink: (Color32, Color32)) {
    let mut state = read(ctx);
    state.name = name;
    state.ink = Some(ink);
    let arrived = ctx.input(|i| {
        i.raw.events.iter().find_map(|event| match event {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    });
    if let Some(image) = arrived {
        state.requested = false;
        if let Some((id, at)) = state.waiting.take() {
            state.picked = pixel_at(&image, at, ctx.pixels_per_point()).map(|rgb| (id, rgb));
        } else if state.armed.is_some() {
            state.shot = Some(image);
        }
        ctx.request_repaint();
    }
    if state.armed.is_some() {
        let leave =
            ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.pointer.secondary_clicked());
        if leave {
            state.armed = None;
            state.shot = None;
        } else {
            if state.shot.is_none() && !state.requested {
                if state.settle > 0 {
                    state.settle -= 1;
                } else {
                    state.requested = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                        egui::UserData::default(),
                    ));
                }
                ctx.request_repaint();
            }
            if let Some(at) = cover(ctx, state.shot.as_deref()) {
                let id = state.armed.take();
                match (id, state.shot.take()) {
                    (Some(id), Some(shot)) => {
                        state.picked =
                            pixel_at(&shot, at, ctx.pixels_per_point()).map(|rgb| (id, rgb));
                    }
                    // Clicked before the screenshot: it is read on arrival.
                    (Some(id), None) => state.waiting = Some((id, at)),
                    _ => {}
                }
                ctx.request_repaint();
            }
        }
    }
    write(ctx, state);
}

/// A transparent layer over the whole window with the crosshair and, once
/// the screenshot is there, the loupe; the point clicked on it, if any.
fn cover(ctx: &egui::Context, shot: Option<&ColorImage>) -> Option<Pos2> {
    let screen = ctx.content_rect();
    let area = egui::Area::new(Id::new("eyedropper.cover"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            let (_, response) = ui.allocate_exact_size(screen.size(), Sense::click());
            let hover = response.hover_pos();
            if let (Some(shot), Some(at)) = (shot, hover) {
                loupe(ui.painter(), shot, at, ui.ctx().pixels_per_point());
            }
            // The crosshair stays: its point is the middle of the loupe.
            let cursor = egui::CursorIcon::Crosshair;
            response
                .on_hover_cursor(cursor)
                .clicked()
                .then(|| ui.ctx().pointer_interact_pos())
                .flatten()
        });
    ctx.move_to_top(area.response.layer_id);
    if shot.is_some() {
        // The loupe follows the pointer.
        ctx.request_repaint();
    }
    area.inner
}

/// How many pixels across the loupe shows, an odd number around the one
/// taken, and how large each one is drawn.
const LOUPE_PIXELS: i32 = 11;
const LOUPE_CELL: f32 = 9.0;

/// The pixels of `shot` around `at`, `LOUPE_PIXELS` across, row by row;
/// outside the image they are none.
pub(crate) fn around(shot: &ColorImage, at: Pos2, ppp: f32) -> Vec<Option<Color32>> {
    let half = LOUPE_PIXELS / 2;
    // Inside the window: the casts only drop the fraction.
    let (cx, cy) = ((at.x * ppp).floor() as i64, (at.y * ppp).floor() as i64);
    let [w, h] = shot.size;
    let mut out = Vec::with_capacity((LOUPE_PIXELS * LOUPE_PIXELS) as usize);
    for j in -half..=half {
        for i in -half..=half {
            let (x, y) = (cx + i64::from(i), cy + i64::from(j));
            let inside = x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h;
            out.push(inside.then(|| shot.pixels[y as usize * w + x as usize]));
        }
    }
    out
}

/// The loupe: a circle of enlarged pixels on a grid, the one taken framed in
/// the middle, and its code under it.
fn loupe(painter: &egui::Painter, shot: &ColorImage, at: Pos2, ppp: f32) {
    let side = LOUPE_PIXELS as f32 * LOUPE_CELL;
    let radius = side / 2.0;
    let centre = at;
    let origin = centre - Vec2::splat(radius);
    // The pixels as a small texture, drawn sharp on a disc.
    let n = LOUPE_PIXELS as usize;
    let pixels = around(shot, at, ppp)
        .into_iter()
        .map(|p| p.unwrap_or(Color32::from_gray(128)))
        .collect();
    let texture = painter.ctx().load_texture(
        "eyedropper.loupe",
        ColorImage::new([n, n], pixels),
        egui::TextureOptions::NEAREST,
    );
    let mut disc = egui::Mesh::with_texture(texture.id());
    const SIDES: u32 = 48;
    disc.vertices.push(egui::epaint::Vertex {
        pos: centre,
        uv: Pos2::new(0.5, 0.5),
        color: Color32::WHITE,
    });
    for k in 0..=SIDES {
        let angle = k as f32 / SIDES as f32 * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        disc.vertices.push(egui::epaint::Vertex {
            pos: centre + Vec2::new(cos, sin) * radius,
            uv: Pos2::new(0.5 + cos / 2.0, 0.5 + sin / 2.0),
            color: Color32::WHITE,
        });
        if k > 0 {
            disc.add_triangle(0, k, k + 1);
        }
    }
    painter.add(disc);
    // The grid, each line cut where it meets the circle.
    let grid = Stroke::new(0.5, Color32::from_black_alpha(60));
    for k in 1..LOUPE_PIXELS {
        let d = k as f32 * LOUPE_CELL - radius;
        let half = (radius * radius - d * d).max(0.0).sqrt();
        painter.line_segment(
            [centre + Vec2::new(d, -half), centre + Vec2::new(d, half)],
            grid,
        );
        painter.line_segment(
            [centre + Vec2::new(-half, d), centre + Vec2::new(half, d)],
            grid,
        );
    }
    painter.circle_stroke(centre, radius, Stroke::new(3.0, Color32::WHITE));
    painter.circle_stroke(
        centre,
        radius + 1.5,
        Stroke::new(1.0, Color32::from_black_alpha(140)),
    );
    let middle =
        Rect::from_center_size(origin + Vec2::splat(radius), Vec2::splat(LOUPE_CELL + 2.0));
    painter.rect_stroke(
        middle,
        1.0,
        Stroke::new(2.0, Color32::WHITE),
        egui::StrokeKind::Middle,
    );
    painter.rect_stroke(
        middle.expand(1.5),
        1.0,
        Stroke::new(1.0, Color32::BLACK),
        egui::StrokeKind::Middle,
    );
    if let Some(rgb) = pixel_at(shot, at, ppp) {
        let text = hex_of([rgb[0], rgb[1], rgb[2], 255]);
        let font = egui::FontId::monospace(12.0);
        let galley = painter.layout_no_wrap(text, font, Color32::WHITE);
        let tag = Rect::from_center_size(
            centre + Vec2::new(0.0, radius + 16.0),
            galley.size() + Vec2::new(16.0, 8.0),
        );
        painter.rect_filled(tag, 6.0, Color32::from_black_alpha(200));
        let dot = Pos2::new(tag.left() - 12.0, tag.center().y);
        painter.circle(
            dot,
            7.0,
            Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
            Stroke::new(2.0, Color32::WHITE),
        );
        painter.galley(tag.center() - galley.size() / 2.0, galley, Color32::WHITE);
    }
}

/// The colour taken for the picker `id`, once.
fn take(ctx: &egui::Context, id: Id) -> Option<[u8; 3]> {
    let mut state = read(ctx);
    match state.picked {
        Some((for_id, rgb)) if for_id == id => {
            state.picked = None;
            write(ctx, state);
            Some(rgb)
        }
        _ => None,
    }
}

/// A round button of `side` points: its rim, and its hover and armed fill.
fn round(ui: &mut Ui, side: f32) -> (Rect, Response) {
    ui.allocate_exact_size(Vec2::splat(side), Sense::click())
}

/// The eyedropper button of the picker `id`: a press arms (or disarms) it.
fn button(ui: &mut Ui, id: Id, side: f32) -> Response {
    let state = read(ui.ctx());
    let armed = state.armed == Some(id);
    let (rect, response) = round(ui, side);
    let (ink, on) = state
        .ink
        .unwrap_or((ui.visuals().text_color(), ui.visuals().selection.bg_fill));
    let visuals = ui.visuals();
    let fill = if armed {
        on
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        visuals.extreme_bg_color
    };
    let rim = visuals.widgets.noninteractive.bg_stroke.color;
    let painter = ui.painter();
    painter.circle(rect.center(), side / 2.0 - 0.5, fill, Stroke::new(1.0, rim));
    let icon = Rect::from_center_size(rect.center(), Vec2::splat(side * 0.55));
    icons::paint(painter, icon, Icon::Eyedropper, Weight::Regular, ink);
    let name = state.name.clone();
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), armed, &name)
    });
    if response.clicked() {
        let mut state = read(ui.ctx());
        state.armed = if armed { None } else { Some(id) };
        state.waiting = None;
        state.shot = None;
        state.settle = 2;
        write(ui.ctx(), state);
        // An open picker would be in the screenshot, over what is taken.
        egui::Popup::close_all(ui.ctx());
    }
    response.on_hover_text(name)
}

/// A circle filled with `rgba` (unmultiplied), over a checkerboard when it is
/// see-through; a cross when it is fully transparent.
fn swatch(painter: &egui::Painter, rect: Rect, rgba: [u8; 4], rim: Stroke) {
    let centre = rect.center();
    let r = rect.width().min(rect.height()) / 2.0 - 0.5;
    if rgba[3] < 255 {
        painter.circle_filled(centre, r, Color32::WHITE);
        let cell = (r / 2.5).max(2.0);
        let clip = painter.with_clip_rect(rect);
        let mut y = rect.top();
        let mut row = 0;
        while y < rect.bottom() {
            let mut x = rect.left() + if row % 2 == 0 { 0.0 } else { cell };
            while x < rect.right() {
                let square = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(cell));
                if square.center().distance(centre) < r {
                    clip.rect_filled(square, 0.0, Color32::from_gray(204));
                }
                x += cell * 2.0;
            }
            y += cell;
            row += 1;
        }
    }
    let color = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
    painter.circle(centre, r, color, rim);
    if rgba[3] == 0 {
        let d = r * std::f32::consts::FRAC_1_SQRT_2;
        painter.line_segment(
            [centre + Vec2::new(-d, d), centre + Vec2::new(d, -d)],
            Stroke::new(1.5, Color32::from_rgb(235, 87, 87)),
        );
    }
}

/// `rgba` as «#RRGGBB».
pub(crate) fn hex_of(rgba: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2])
}

/// The colour written as «#RGB» or «#RRGGBB» (the «#» optional, any case).
pub(crate) fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let digits = text.trim().trim_start_matches('#');
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match digits.len() {
        3 => {
            let mut out = [0; 3];
            for (n, c) in digits.chars().enumerate() {
                let v = byte(&c.to_string())?;
                out[n] = v * 17;
            }
            Some(out)
        }
        6 => Some([
            byte(&digits[0..2])?,
            byte(&digits[2..4])?,
            byte(&digits[4..6])?,
        ]),
        _ => None,
    }
}

/// A colour picker (`rgba` in unmultiplied sRGB bytes): a round swatch that
/// opens the picker (the colour square, the hue and transparency tracks, the
/// hexadecimal code and a second eyedropper), with the eyedropper beside it.
/// The response is the swatch's, marked changed when a colour was chosen,
/// typed or taken this frame.
pub(crate) fn color_edit(ui: &mut Ui, rgba: &mut [u8; 4]) -> Response {
    color_edit_opening(ui, rgba, false)
}

/// [`color_edit`], its picker opened now when `open` (the colour just added
/// to a gradient, spec 063).
pub(crate) fn color_edit_opening(ui: &mut Ui, rgba: &mut [u8; 4], open: bool) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let side = ui.spacing().interact_size.y.max(24.0);
        let (rect, mut response) = round(ui, side);
        let base = response.id;
        let popup_id = base.with("popup");
        let inner_id = base.with("eyedropper.in");
        if open {
            egui::Popup::open_id(ui.ctx(), popup_id);
        }
        let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        if let Some(rgb) = take(ui.ctx(), base.with("eyedropper")) {
            *rgba = with_taken(*rgba, rgb);
            response.mark_changed();
        }
        if let Some(rgb) = take(ui.ctx(), inner_id) {
            *rgba = with_taken(*rgba, rgb);
            response.mark_changed();
            // Taken from inside the picker: it stays open.
            egui::Popup::open_id(ui.ctx(), popup_id);
        }
        let visuals = ui.visuals();
        let rim = if open || response.hovered() {
            Stroke::new(2.0, visuals.selection.stroke.color)
        } else {
            visuals.widgets.noninteractive.bg_stroke
        };
        swatch(ui.painter(), rect, *rgba, rim);
        let hex = hex_of(*rgba);
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ColorButton, true, &hex));
        let response = response.on_hover_text(hex);
        button(ui, base.with("eyedropper"), side);
        let mut edited = false;
        egui::Popup::menu(&response)
            .id(popup_id)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                edited = picker(ui, rgba, popup_id, inner_id);
            });
        let mut response = response;
        if edited {
            response.mark_changed();
        }
        response
    })
    .inner
}

/// The inside of the picker: the colour square and tracks, then the swatch,
/// the hexadecimal code and the eyedropper; whether the colour changed.
fn picker(ui: &mut Ui, rgba: &mut [u8; 4], popup_id: Id, dropper: Id) -> bool {
    use egui::ecolor::Hsva;
    ui.set_width(PICKER_WIDTH);
    ui.spacing_mut().item_spacing.y = 10.0;
    // The hue is kept while the colour is grey, from frame to frame.
    let hsva_id = popup_id.with("hsva");
    let stored = ui.data(|d| d.get_temp::<Hsva>(hsva_id));
    let mut hsva = stored
        .filter(|h| h.to_srgba_unmultiplied() == *rgba)
        .unwrap_or_else(|| Hsva::from_srgba_unmultiplied(*rgba));
    let before = hsva;
    saturation_value(ui, &mut hsva);
    let hues: Vec<Color32> = (0..=12)
        .map(|n| Color32::from(Hsva::new(n as f32 / 12.0, 1.0, 1.0, 1.0)))
        .collect();
    let knob = Color32::from(Hsva::new(hsva.h, 1.0, 1.0, 1.0));
    bar(ui, &mut hsva.h, &hues, knob, false);
    let solid = Color32::from(Hsva { a: 1.0, ..hsva });
    let fade = [solid.gamma_multiply(0.0), solid];
    let shown = Color32::from(hsva);
    bar(ui, &mut hsva.a, &fade, shown, true);
    let mut changed = hsva != before;
    if changed {
        *rgba = hsva.to_srgba_unmultiplied();
    }
    ui.data_mut(|d| d.insert_temp(hsva_id, hsva));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let side = ui.spacing().interact_size.y.max(24.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
        let rim = ui.visuals().widgets.noninteractive.bg_stroke;
        swatch(ui.painter(), rect, *rgba, rim);
        // What is typed is kept while it is being written.
        let text_id = popup_id.with("hex");
        let mut text = ui
            .data(|d| d.get_temp::<String>(text_id))
            .unwrap_or_else(|| hex_of(*rgba));
        let edit = ui.add_sized(
            Vec2::new(PICKER_WIDTH - 2.0 * side - 16.0, side),
            egui::TextEdit::singleline(&mut text)
                .font(egui::TextStyle::Monospace)
                .char_limit(7),
        );
        if edit.changed()
            && let Some(rgb) = parse_hex(&text)
        {
            *rgba = with_taken(*rgba, rgb);
            changed = true;
        }
        if edit.has_focus() {
            ui.data_mut(|d| d.insert_temp(text_id, text));
        } else {
            ui.data_mut(|d| d.remove::<String>(text_id));
        }
        button(ui, dropper, side);
    });
    changed
}

/// The width of the inside of the picker.
const PICKER_WIDTH: f32 = 260.0;

/// The square of saturation (across) and brightness (up) of the hue.
fn saturation_value(ui: &mut Ui, hsva: &mut egui::ecolor::Hsva) {
    use egui::ecolor::Hsva;
    let size = Vec2::new(PICKER_WIDTH, 150.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    if let Some(at) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked())
    {
        hsva.s = ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        hsva.v = 1.0 - ((at.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    }
    let painter = ui.painter_at(rect);
    // White to the pure hue across, then clear to black down; a grid, so
    // the blend follows the colour and not only its corners.
    const CELLS: u32 = 16;
    let mut mesh = egui::Mesh::default();
    for j in 0..=CELLS {
        for i in 0..=CELLS {
            let (u, v) = (i as f32 / CELLS as f32, j as f32 / CELLS as f32);
            let at = Pos2::new(
                rect.left() + u * rect.width(),
                rect.top() + v * rect.height(),
            );
            mesh.colored_vertex(at, Color32::from(Hsva::new(hsva.h, u, 1.0 - v, 1.0)));
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
    painter.add(mesh);
    let knob = Pos2::new(
        rect.left() + hsva.s * rect.width(),
        rect.top() + (1.0 - hsva.v) * rect.height(),
    );
    let fill = Color32::from(Hsva { a: 1.0, ..*hsva });
    painter.circle(knob, 7.0, fill, Stroke::new(2.0, Color32::WHITE));
    painter.circle_stroke(knob, 8.0, Stroke::new(1.0, Color32::from_black_alpha(90)));
}

/// A round-ended track of `colors` with its knob filled with `knob`;
/// `value` from 0 to 1 along it; a checkerboard under it for transparency.
fn bar(ui: &mut Ui, value: &mut f32, colors: &[Color32], knob: Color32, checkers: bool) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(PICKER_WIDTH, 16.0), Sense::click_and_drag());
    let room = rect.shrink2(Vec2::new(8.0, 0.0));
    if let Some(at) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked())
    {
        *value = ((at.x - room.left()) / room.width()).clamp(0.0, 1.0);
    }
    let painter = ui.painter();
    let track = rect.shrink2(Vec2::new(0.0, 2.0));
    if checkers {
        let clip = painter.with_clip_rect(track);
        let cell = track.height() / 2.0;
        let mut n = 0;
        let mut x = track.left();
        while x < track.right() {
            for k in 0..2 {
                let gray = if (n + k) % 2 == 0 { 255 } else { 204 };
                let square = Rect::from_min_size(
                    Pos2::new(x, track.top() + k as f32 * cell),
                    Vec2::splat(cell),
                );
                clip.rect_filled(square, 0.0, Color32::from_gray(gray));
            }
            x += cell;
            n += 1;
        }
    }
    let mut mesh = egui::Mesh::default();
    let last = (colors.len().max(2) - 1) as f32;
    for (n, color) in colors.iter().enumerate() {
        let x = track.left() + track.width() * n as f32 / last;
        mesh.colored_vertex(Pos2::new(x, track.top()), *color);
        mesh.colored_vertex(Pos2::new(x, track.bottom()), *color);
        if n > 0 {
            let at = n as u32 * 2;
            mesh.add_triangle(at - 2, at - 1, at);
            mesh.add_triangle(at - 1, at, at + 1);
        }
    }
    painter.add(mesh);
    let at = Pos2::new(room.left() + *value * room.width(), rect.center().y);
    painter.circle(at, 8.0, knob, Stroke::new(2.5, Color32::WHITE));
    painter.circle_stroke(at, 9.0, Stroke::new(1.0, Color32::from_black_alpha(90)));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

/// [`color_edit`] for an opaque colour, as `Color32`.
pub(crate) fn color_edit_opaque(ui: &mut Ui, color: &mut Color32) -> Response {
    let mut rgba = [color.r(), color.g(), color.b(), 255];
    let response = color_edit(ui, &mut rgba);
    if response.changed() {
        *color = Color32::from_rgb(rgba[0], rgba[1], rgba[2]);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> ColorImage {
        // 4 × 2 pixels, each a different red.
        let pixels = (0..8u8).map(|n| Color32::from_rgb(n * 10, 1, 2)).collect();
        ColorImage::new([4, 2], pixels)
    }

    #[test]
    fn s98_01_the_pixel_under_the_pointer_is_taken_at_the_scale_of_the_screen() {
        let image = image();
        assert_eq!(pixel_at(&image, Pos2::new(0.2, 0.2), 1.0), Some([0, 1, 2]));
        assert_eq!(pixel_at(&image, Pos2::new(3.9, 1.0), 1.0), Some([70, 1, 2]));
        // Two pixels per point: (1, 0.5) points is pixel (2, 1).
        assert_eq!(pixel_at(&image, Pos2::new(1.0, 0.5), 2.0), Some([60, 1, 2]));
    }

    #[test]
    fn s98_01_outside_the_window_nothing_is_taken() {
        let image = image();
        assert_eq!(pixel_at(&image, Pos2::new(4.0, 0.0), 1.0), None);
        assert_eq!(pixel_at(&image, Pos2::new(0.0, 2.0), 1.0), None);
        assert_eq!(pixel_at(&image, Pos2::new(-0.5, 0.0), 1.0), None);
        assert_eq!(pixel_at(&image, Pos2::new(f32::NAN, 0.0), 1.0), None);
    }

    #[test]
    fn s98_01_the_loupe_shows_the_pixels_around_the_one_taken() {
        let image = image();
        let cells = around(&image, Pos2::new(1.0, 0.0), 1.0);
        let n = LOUPE_PIXELS as usize;
        assert_eq!(cells.len(), n * n);
        // The middle is the pixel taken; its right neighbour follows.
        let middle = n * n / 2;
        assert_eq!(cells[middle], Some(Color32::from_rgb(10, 1, 2)));
        assert_eq!(cells[middle + 1], Some(Color32::from_rgb(20, 1, 2)));
        // Above the window there is nothing.
        assert_eq!(cells[middle - n], None);
    }

    #[test]
    fn s98_02_the_taken_colour_keeps_the_transparency_of_the_picker() {
        assert_eq!(with_taken([1, 2, 3, 128], [9, 8, 7]), [9, 8, 7, 128]);
        assert_eq!(with_taken([1, 2, 3, 255], [9, 8, 7]), [9, 8, 7, 255]);
        // A colour that could not be seen becomes opaque.
        assert_eq!(with_taken([0, 0, 0, 0], [9, 8, 7]), [9, 8, 7, 255]);
    }

    #[test]
    fn s98_03_a_colour_is_handed_only_to_the_picker_that_asked_for_it() {
        let ctx = egui::Context::default();
        let (mine, other) = (Id::new("mine"), Id::new("other"));
        write(
            &ctx,
            State {
                picked: Some((mine, [5, 6, 7])),
                ..State::default()
            },
        );
        assert_eq!(take(&ctx, other), None);
        assert_eq!(take(&ctx, mine), Some([5, 6, 7]));
        // Once.
        assert_eq!(take(&ctx, mine), None);
    }

    #[test]
    fn s98_03_a_screenshot_hands_the_clicked_pixel_to_the_waiting_picker() {
        let ctx = egui::Context::default();
        let id = Id::new("picker");
        write(
            &ctx,
            State {
                waiting: Some((id, Pos2::new(2.0, 1.0))),
                ..State::default()
            },
        );
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Screenshot {
            viewport_id: egui::ViewportId::ROOT,
            user_data: egui::UserData::default(),
            image: std::sync::Arc::new(image()),
        });
        let _ = ctx.run(input, |ctx| {
            frame(ctx, String::new(), (Color32::BLACK, Color32::WHITE));
        });
        assert_eq!(read(&ctx).waiting, None);
        assert_eq!(take(&ctx, id), Some([60, 1, 2]));
    }

    #[test]
    fn s98_05_the_code_of_a_colour_is_read_and_written_in_hexadecimal() {
        assert_eq!(hex_of([0x61, 0x84, 0x4C, 255]), "#61844C");
        assert_eq!(parse_hex("#61844c"), Some([0x61, 0x84, 0x4C]));
        assert_eq!(parse_hex(" 61844C "), Some([0x61, 0x84, 0x4C]));
        assert_eq!(parse_hex("#f80"), Some([0xFF, 0x88, 0x00]));
        assert_eq!(parse_hex("#61844"), None);
        assert_eq!(parse_hex("#GG0000"), None);
        assert_eq!(parse_hex("#ÿÿÿ"), None);
    }

    #[test]
    fn s98_01_a_click_with_the_loupe_takes_the_pixel_at_once() {
        let ctx = egui::Context::default();
        let id = Id::new("picker");
        write(
            &ctx,
            State {
                armed: Some(id),
                shot: Some(std::sync::Arc::new(image())),
                ..State::default()
            },
        );
        let at = Pos2::new(2.0, 1.0);
        let run = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
                ..egui::RawInput::default()
            };
            let _ = ctx.run(input, |ctx| {
                frame(ctx, String::new(), (Color32::BLACK, Color32::WHITE));
            });
        };
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(vec![egui::Event::PointerMoved(at)]);
        run(vec![egui::Event::PointerMoved(at)]);
        run(vec![button(true)]);
        run(vec![button(false)]);
        assert_eq!(read(&ctx).armed, None);
        assert_eq!(take(&ctx, id), Some([60, 1, 2]));
    }

    #[test]
    fn s98_04_escape_leaves_the_dropper_without_a_colour() {
        let ctx = egui::Context::default();
        let id = Id::new("picker");
        write(
            &ctx,
            State {
                armed: Some(id),
                ..State::default()
            },
        );
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = ctx.run(input, |ctx| {
            frame(ctx, String::new(), (Color32::BLACK, Color32::WHITE));
        });
        let state = read(&ctx);
        assert_eq!(
            (state.armed, state.waiting, state.picked),
            (None, None, None)
        );
    }
}
