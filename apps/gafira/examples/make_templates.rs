//! Writes the templates Gafira carries (spec 091) into `assets/templates/`:
//! each design in Spanish and in English, drawn with the render of the
//! exports for its thumbnail and saved as a `.gaf` with the `template` of
//! D-81. Run from the repository root:
//!
//! ```text
//! cargo run -p gafira --example make_templates
//! ```
//!
//! More templates need no code: a `.gaf` saved with a `template` object in
//! any folder under `assets/templates/` is listed too.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{self, Cursor};
use std::path::Path;
use std::sync::{Arc, Mutex};

use gafira_doc::contract::CancelToken;
use gafira_doc::{ContentHash, Document};
use gafira_format::write_template;
use gafira_geom::IRect;
use gafira_ports::export::{PageRenderRequest, PageRenderers};
use gafira_ports::repository::{THUMBNAIL_SIDE, TemplateMeta, Thumbnail};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use gafira_render::{FontBook, GraphPageRenderers};
use gafira_render_cpu::CpuBackend;
use gafira_time::Flicks;
use serde_json::{Value, json};

// ---------------------------------------------------------------- building

/// Px per mm and per inch at Gafira's 96 dpi.
const MM: f64 = 96.0 / 25.4;
const INCH: f64 = 96.0;

/// A colour from `#RRGGBB` and an alpha.
fn rgba(hex: &str, a: f64) -> Value {
    let n = u32::from_str_radix(hex.trim_start_matches('#'), 16).expect("a hex colour");
    let c = |shift: u32| f64::from((n >> shift) & 0xFF) / 255.0;
    json!({ "space": "srgb", "r": c(16), "g": c(8), "b": c(0), "a": a })
}

fn rgb(hex: &str) -> Value {
    rgba(hex, 1.0)
}

/// A linear gradient from `from` to `to` at `angle` degrees.
fn linear(angle: f64, from: &str, to: &str) -> Value {
    json!({
        "kind": { "linear": { "angle_deg": angle } },
        "stops": [
            { "offset": 0.0, "color": rgb(from) },
            { "offset": 1.0, "color": rgb(to) }
        ]
    })
}

/// How a text looks.
#[derive(Clone, Copy)]
struct Font {
    family: &'static str,
    size: f64,
    color: &'static str,
    bold: bool,
    italic: bool,
    align: &'static str,
    /// Line height, as a factor of the size.
    line: f64,
    spacing: f64,
}

const fn font(family: &'static str, size: f64, color: &'static str) -> Font {
    Font {
        family,
        size,
        color,
        bold: false,
        italic: false,
        align: "left",
        line: 1.2,
        spacing: 0.0,
    }
}

impl Font {
    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    const fn right(mut self) -> Self {
        self.align = "right";
        self
    }
    const fn center(mut self) -> Self {
        self.align = "center";
        self
    }
    const fn line(mut self, line: f64) -> Self {
        self.line = line;
        self
    }
    const fn spaced(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }
}

/// One page while it is drawn: its size in px and its elements.
struct Page {
    w: f64,
    h: f64,
    size: Value,
    background: Value,
    nodes: Vec<Value>,
    roots: Vec<u64>,
}

/// The document while it is built: the ids so far and its pages.
struct Doc {
    next: u64,
    pages: Vec<Value>,
}

impl Doc {
    fn new() -> Self {
        Self {
            next: 1,
            pages: Vec::new(),
        }
    }

    /// A page of `w` × `h` in `unit` («px», «mm», «in»), filled by `draw`.
    fn page(
        &mut self,
        (w, h, unit): (f64, f64, &str),
        background: Value,
        draw: impl FnOnce(&mut Self, &mut Page),
    ) {
        let k = match unit {
            "mm" => MM,
            "in" => INCH,
            _ => 1.0,
        };
        let length = |v: f64| json!({ "value": v, "unit": unit });
        let mut page = Page {
            w: w * k,
            h: h * k,
            size: json!({ "width": length(w), "height": length(h) }),
            background,
            nodes: Vec::new(),
            roots: Vec::new(),
        };
        let id = self.id();
        draw(self, &mut page);
        let zero = json!({ "value": 0.0, "unit": "px" });
        self.pages.push(json!({
            "id": id,
            "name": "",
            "size": page.size,
            "bleed": { "value": 0.0, "unit": "mm" },
            "margins": { "top": zero, "right": zero, "bottom": zero, "left": zero, "linked": true },
            "guides": [],
            "grid": { "visible": false, "step": { "value": 10.0, "unit": "px" }, "subdivisions": 1, "style": "lines", "snap": true },
            "locked": false,
            "hidden": false,
            "background": page.background,
            "roots": page.roots,
            "nodes": page.nodes,
        }));
    }

    fn id(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }

    fn add(
        &mut self,
        page: &mut Page,
        name: &str,
        (x, y, w, h): (f64, f64, f64, f64),
        kind: Value,
        opacity: f64,
    ) {
        let id = self.id();
        page.roots.push(id);
        page.nodes.push(json!({
            "base": {
                "id": id,
                "name": name,
                "placement": { "x": x, "y": y, "w": w, "h": h, "rotation_deg": 0.0, "skew": 0.0, "flip": { "x": false, "y": false } },
                "opacity": opacity,
                "blend": "normal",
                "visible": true,
                "locked": false
            },
            "kind": kind,
            "children": []
        }));
    }

    /// A rectangle with rounded corners of `radius`.
    fn rect(&mut self, page: &mut Page, at: (f64, f64, f64, f64), fill: &str, radius: f64) {
        let kind = json!({ "type": "shape", "geometry": "rect", "fill": rgb(fill), "corner_radius": radius });
        self.add(page, "Rectángulo", at, kind, 1.0);
    }

    /// An outlined rectangle.
    fn frame(
        &mut self,
        page: &mut Page,
        at: (f64, f64, f64, f64),
        color: &str,
        width: f64,
        radius: f64,
    ) {
        let kind = json!({
            "type": "shape", "geometry": "rect", "fill": null,
            "stroke": { "color": rgb(color), "width": width }, "corner_radius": radius
        });
        self.add(page, "Marco", at, kind, 1.0);
    }

    fn ellipse(&mut self, page: &mut Page, at: (f64, f64, f64, f64), fill: &str, opacity: f64) {
        let kind = json!({ "type": "shape", "geometry": "ellipse", "fill": rgb(fill) });
        self.add(page, "Elipse", at, kind, opacity);
    }

    /// A closed path through `points` of the unit box.
    fn polygon(
        &mut self,
        page: &mut Page,
        at: (f64, f64, f64, f64),
        fill: &str,
        points: &[(f64, f64)],
        opacity: f64,
    ) {
        let mut commands = vec![json!({ "move_to": [points[0].0, points[0].1] })];
        commands.extend(
            points[1..]
                .iter()
                .map(|(x, y)| json!({ "line_to": [x, y] })),
        );
        commands.push(json!("close"));
        let kind = json!({ "type": "shape", "geometry": { "path": { "commands": commands } }, "fill": rgb(fill) });
        self.add(page, "Forma", at, kind, opacity);
    }

    /// The shape of the Formas panel named `key` (spec 098), `h` high from
    /// `(x, y)` and as wide as its proportions ask.
    fn piece(&mut self, page: &mut Page, (x, y, h): (f64, f64, f64), key: &str, fill: &str) {
        let (geometry, aspect) = gafira_ui::piece_geometry(key).expect("a shape of the panel");
        let geometry = serde_json::to_value(geometry).expect("geometry as JSON");
        let kind = json!({ "type": "shape", "geometry": geometry, "fill": rgb(fill) });
        self.add(page, "Comillas", (x, y, h * aspect, h), kind, 1.0);
    }

    /// An outlined ellipse.
    fn ring(&mut self, page: &mut Page, at: (f64, f64, f64, f64), color: &str, width: f64) {
        let kind = json!({
            "type": "shape", "geometry": "ellipse", "fill": null,
            "stroke": { "color": rgb(color), "width": width }
        });
        self.add(page, "Anillo", at, kind, 1.0);
    }

    /// An empty frame of `shape` («rect» or «ellipse»): a photo dropped on
    /// it fills it and keeps the design (spec 066). An empty frame does not
    /// print, so a placeholder goes under it.
    fn photo_frame(&mut self, page: &mut Page, at: (f64, f64, f64, f64), shape: &str, radius: f64) {
        let kind = json!({
            "type": "frame",
            "clip_shape": shape,
            "corner_radius": radius,
            "fit": "fill",
            "content_offset": [0.0, 0.0],
            "content_scale": 1.0
        });
        self.add(page, "Marco de foto", at, kind, 1.0);
    }

    /// Outlines the frame added last with `color`, `width` px wide: the
    /// frame's own outline, so no other element covers it.
    fn outline_last(page: &mut Page, color: &str, width: f64) {
        if let Some(node) = page.nodes.last_mut() {
            node["kind"]["stroke"] = json!({ "color": rgb(color), "width": width });
        }
    }

    /// Turns the element added last by `degrees`.
    fn turn_last(page: &mut Page, degrees: f64) {
        if let Some(node) = page.nodes.last_mut() {
            node["base"]["placement"]["rotation_deg"] = json!(degrees);
        }
    }

    /// A burst of `points` rays whose inner corners are at `inner` (0–0.5).
    fn burst(
        &mut self,
        page: &mut Page,
        at: (f64, f64, f64, f64),
        fill: &str,
        points: u32,
        inner: f64,
    ) {
        let n = points * 2;
        let corners: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let r = if i % 2 == 0 { 0.5 } else { inner };
                let a = std::f64::consts::TAU * f64::from(i) / f64::from(n)
                    - std::f64::consts::FRAC_PI_2;
                (0.5 + r * a.cos(), 0.5 + r * a.sin())
            })
            .collect();
        self.polygon(page, at, fill, &corners, 1.0);
    }

    /// A five-pointed star.
    fn star(&mut self, page: &mut Page, at: (f64, f64, f64, f64), fill: &str) {
        let points: Vec<(f64, f64)> = (0..10)
            .map(|i| {
                let r = if i % 2 == 0 { 0.5 } else { 0.2 };
                let a = std::f64::consts::PI * (f64::from(i) / 5.0) - std::f64::consts::FRAC_PI_2;
                (0.5 + r * a.cos(), 0.5 + r * a.sin())
            })
            .collect();
        self.polygon(page, at, fill, &points, 1.0);
    }

    /// A text of one paragraph per line of `text`, `w` wide, from `(x, y)`.
    fn text(&mut self, page: &mut Page, (x, y, w): (f64, f64, f64), text: &str, f: Font) {
        let lines: Vec<&str> = text.split('\n').collect();
        let style = json!({ "font": f.family, "size": f.size, "color": rgb(f.color), "bold": f.bold, "italic": f.italic });
        let paragraphs: Vec<Value> = lines
            .iter()
            .map(|line| json!({ "spans": [{ "text": line, "style": style }] }))
            .collect();
        // Most faces are about 1.35 times their size from line to line.
        let h = (lines.len() as f64 * f.size * f.line * 1.35).max(f.size);
        let mut kind = json!({
            "type": "text",
            "content": { "paragraphs": paragraphs },
            "sizing": "auto_height",
            "align": f.align,
            "line_height": f.line
        });
        if f.spacing != 0.0 {
            kind["letter_spacing"] = json!(f.spacing);
        }
        let name: String = text.chars().take(24).collect();
        self.add(page, &name, (x, y, w, h), kind, 1.0);
    }

    fn document(self) -> Document {
        let value = json!({
            "schema_version": Document::SCHEMA_VERSION,
            "next_id": self.next,
            "project_id": "00000000-0000-0000-0000-000000000000",
            "dpi": 96,
            "pages": self.pages,
            "sequences": [],
            "assets": {}
        });
        Document::from_json(value.to_string().as_bytes()).expect("a valid template")
    }
}

fn solid(hex: &str) -> Value {
    json!({ "solid": rgb(hex) })
}

fn pattern(kind: &str, color: &str, back: &str) -> Value {
    json!({ "pattern": { "kind": kind, "color": rgb(color), "back": rgb(back) } })
}

// ------------------------------------------------------------------ designs

/// The language a template is written in.
#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Es,
    En,
}

impl Lang {
    fn pick<'a>(self, es: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::Es => es,
            Self::En => en,
        }
    }
}

/// One design: its folder, names, kind, format, style, colour and drawing.
struct Design {
    folder: &'static str,
    /// The file's name, in English; `.es.gaf` and `.en.gaf` follow it.
    file: &'static str,
    names: (&'static str, &'static str),
    kind: &'static str,
    page_format: &'static str,
    style: &'static str,
    color: &'static str,
    tags: &'static [&'static str],
    draw: fn(Lang) -> Document,
}

const INK: &str = "#18243B";
const CORAL: &str = "#FF6B57";
const CANVAS: &str = "#FAFAF7";
const MINT: &str = "#3FBFA9";
const MIST: &str = "#E7EAF2";
const WHITE: &str = "#FFFFFF";
// Tones of the brand, from the theme's tokens (spec 03 §3).
const INK_LIGHT: &str = "#22325A";
const SLATE: &str = "#46526B";
const BORDER: &str = "#C3CADB";
const CORAL_LIGHT: &str = "#FF7E6C";
const MINT_DARK: &str = "#17695A";
const MINT_PALE: &str = "#E3F5F1";
const MINT_TINT: &str = "#CFEFE8";

fn cafe(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((1080.0, 1080.0, "px"), solid(CANVAS), |d, p| {
        d.ellipse(p, (620.0, -180.0, 640.0, 640.0), MINT_TINT, 1.0);
        d.ellipse(p, (-160.0, 760.0, 480.0, 480.0), CORAL, 0.35);
        d.rect(p, (90.0, 90.0, 900.0, 900.0), "#FFFFFF", 36.0);
        d.ellipse(p, (430.0, 150.0, 220.0, 220.0), INK, 1.0);
        d.ellipse(p, (470.0, 190.0, 140.0, 140.0), CORAL, 1.0);
        d.text(
            p,
            (140.0, 400.0, 800.0),
            l.pick("Café de barrio", "Corner Coffee"),
            font("Pacifico", 96.0, INK).center(),
        );
        d.text(
            p,
            (140.0, 600.0, 800.0),
            l.pick("NUEVA CARTA DE OTOÑO", "NEW AUTUMN MENU"),
            font("Montserrat", 38.0, INK).bold().center().spaced(6.0),
        );
        d.rect(p, (390.0, 670.0, 300.0, 4.0), CORAL, 2.0);
        d.text(
            p,
            (140.0, 700.0, 800.0),
            l.pick(
                "Lattes de especias, pan de calabaza\ny pastelería recién horneada.",
                "Spiced lattes, pumpkin bread\nand freshly baked pastries.",
            ),
            font("Lato", 30.0, SLATE).center().line(1.4),
        );
        d.rect(p, (360.0, 860.0, 360.0, 80.0), INK, 40.0);
        d.text(
            p,
            (360.0, 880.0, 360.0),
            l.pick("Visítanos hoy", "Visit us today"),
            font("Montserrat", 30.0, WHITE).bold().center(),
        );
    });
    d.document()
}

fn sale(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((1200.0, 630.0, "px"), solid(CORAL), |d, p| {
        d.ellipse(p, (780.0, -120.0, 560.0, 560.0), CORAL_LIGHT, 1.0);
        d.ellipse(p, (900.0, 330.0, 420.0, 420.0), MINT, 1.0);
        d.star(p, (90.0, 50.0, 60.0, 60.0), MINT);
        d.text(
            p,
            (90.0, 130.0, 700.0),
            l.pick("GRAN VENTA\nDE VERANO", "BIG SUMMER\nSALE"),
            font("Anton", 104.0, INK).line(0.8),
        );
        d.text(
            p,
            (90.0, 405.0, 620.0),
            l.pick(
                "Hasta 50 % en toda la tienda. Solo este fin de semana.",
                "Up to 50% off everything. This weekend only.",
            ),
            font("Poppins", 28.0, INK).line(1.35),
        );
        d.rect(p, (90.0, 525.0, 260.0, 70.0), INK, 35.0);
        d.text(
            p,
            (90.0, 542.0, 260.0),
            l.pick("Comprar ya", "Shop now"),
            font("Poppins", 28.0, WHITE).bold().center(),
        );
        d.text(
            p,
            (930.0, 470.0, 280.0),
            "-50%",
            font("Archivo Black", 96.0, INK).center(),
        );
    });
    d.document()
}

fn quote(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(
        (1080.0, 1350.0, "px"),
        pattern("dots", BORDER, CANVAS),
        |d, p| {
            d.rect(p, (110.0, 160.0, 860.0, 1030.0), WHITE, 28.0);
            d.frame(p, (140.0, 190.0, 800.0, 970.0), MINT, 3.0, 20.0);
            // The quotes are a shape of the panel, not a letter (spec 098).
            d.piece(p, (488.0, 300.0, 92.0), "pieces.quote_round_open", MINT);
            d.text(
                p,
                (190.0, 500.0, 700.0),
                l.pick(
                    "La creatividad es la inteligencia divirtiéndose.",
                    "Creativity is intelligence having fun.",
                ),
                font("DM Serif Display", 74.0, INK).center().line(1.2),
            );
            d.rect(p, (470.0, 910.0, 140.0, 4.0), CORAL, 2.0);
            d.text(
                p,
                (190.0, 950.0, 700.0),
                l.pick("Frase del día", "Quote of the day"),
                font("Montserrat", 30.0, SLATE).center().spaced(4.0),
            );
        },
    );
    d.document()
}

fn workshop_story(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(
        (1080.0, 1920.0, "px"),
        json!({ "gradient": linear(90.0, INK, MINT_DARK) }),
        |d, p| {
            d.ellipse(p, (-200.0, -200.0, 700.0, 700.0), MINT, 0.35);
            d.ellipse(p, (640.0, 1400.0, 700.0, 700.0), MINT, 0.3);
            d.rect(p, (120.0, 300.0, 300.0, 70.0), MINT, 35.0);
            d.text(
                p,
                (120.0, 315.0, 300.0),
                l.pick("EN VIVO", "LIVE"),
                font("Montserrat", 32.0, INK).bold().center().spaced(6.0),
            );
            d.text(
                p,
                (120.0, 440.0, 840.0),
                l.pick(
                    "Taller de\ndiseño\npara\nprincipiantes",
                    "Design\nworkshop\nfor\nbeginners",
                ),
                font("Poppins", 120.0, WHITE).bold().line(0.8),
            );
            d.text(
                p,
                (120.0, 1120.0, 840.0),
                l.pick(
                    "Aprende a crear tus primeras piezas en una hora.",
                    "Learn to make your first designs in one hour.",
                ),
                font("Inter", 40.0, MIST).line(1.4),
            );
            d.rect(p, (120.0, 1400.0, 840.0, 4.0), MINT, 2.0);
            d.text(
                p,
                (120.0, 1450.0, 840.0),
                l.pick("Jueves · 18:00", "Thursday · 6 PM"),
                font("Poppins", 56.0, WHITE).bold(),
            );
            d.text(
                p,
                (120.0, 1540.0, 840.0),
                l.pick("Desliza para inscribirte", "Swipe up to sign up"),
                font("Inter", 36.0, MINT),
            );
        },
    );
    d.document()
}

fn thumbnail_video(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((1280.0, 720.0, "px"), solid(INK), |d, p| {
        d.polygon(
            p,
            (620.0, 0.0, 660.0, 720.0),
            MINT,
            &[(0.25, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
            1.0,
        );
        d.ellipse(p, (820.0, 160.0, 380.0, 380.0), CORAL, 1.0);
        d.polygon(
            p,
            (960.0, 270.0, 150.0, 170.0),
            WHITE,
            &[(0.0, 0.0), (1.0, 0.5), (0.0, 1.0)],
            1.0,
        );
        d.text(p, (70.0, 20.0, 640.0), "5", font("Anton", 220.0, MINT));
        d.text(
            p,
            (70.0, 360.0, 640.0),
            l.pick(
                "TRUCOS DE\nDISEÑO QUE\nNADIE TE DICE",
                "DESIGN\nTRICKS NOBODY\nTELLS YOU",
            ),
            font("Anton", 84.0, WHITE).line(0.8),
        );
    });
    d.document()
}

fn project_deck(l: Lang) -> Document {
    let mut d = Doc::new();
    let size = (1920.0, 1080.0, "px");
    d.page(size, solid(INK), |d, p| {
        d.rect(p, (0.0, 0.0, 24.0, 1080.0), CORAL, 0.0);
        d.ellipse(p, (1300.0, -260.0, 900.0, 900.0), INK_LIGHT, 1.0);
        d.ellipse(p, (1500.0, 620.0, 300.0, 300.0), MINT, 1.0);
        d.text(
            p,
            (160.0, 300.0, 1200.0),
            l.pick("PROPUESTA 2027", "2027 PROPOSAL"),
            font("Montserrat", 36.0, MINT).bold().spaced(8.0),
        );
        d.text(
            p,
            (160.0, 380.0, 1300.0),
            l.pick("Proyecto\nCreativo", "Creative\nProject"),
            font("Manrope", 170.0, WHITE).bold().line(0.85),
        );
        d.text(
            p,
            (160.0, 800.0, 1200.0),
            l.pick(
                "Presentado por el equipo de diseño",
                "Presented by the design team",
            ),
            font("Inter", 36.0, BORDER),
        );
    });
    d.page(size, solid(CANVAS), |d, p| {
        d.text(
            p,
            (160.0, 140.0, 1600.0),
            l.pick("Agenda", "Agenda"),
            font("Manrope", 96.0, INK).bold(),
        );
        d.rect(p, (160.0, 270.0, 160.0, 8.0), CORAL, 4.0);
        let items = [
            l.pick("Contexto y objetivos", "Context and goals"),
            l.pick("Nuestra propuesta", "Our proposal"),
            l.pick("Calendario", "Timeline"),
            l.pick("Presupuesto", "Budget"),
        ];
        for (i, item) in items.iter().enumerate() {
            let y = 380.0 + i as f64 * 150.0;
            d.ellipse(
                p,
                (160.0, y, 96.0, 96.0),
                if i % 2 == 0 { MINT } else { CORAL },
                1.0,
            );
            d.text(
                p,
                (160.0, y + 22.0, 96.0),
                &format!("0{}", i + 1),
                font("Manrope", 40.0, INK).bold().center(),
            );
            d.text(p, (300.0, y + 20.0, 1300.0), item, font("Inter", 52.0, INK));
        }
    });
    d.page(size, solid(WHITE), |d, p| {
        d.rect(p, (0.0, 0.0, 760.0, 1080.0), MIST, 0.0);
        d.text(
            p,
            (120.0, 160.0, 560.0),
            l.pick("Nuestra\npropuesta", "Our\nproposal"),
            font("Manrope", 96.0, INK).bold().line(1.05),
        );
        d.text(
            p,
            (120.0, 460.0, 540.0),
            l.pick(
                "Un resumen en una frase de lo que haremos y por qué importa.",
                "A one-sentence summary of what we will do and why it matters.",
            ),
            font("Inter", 34.0, SLATE).line(1.45),
        );
        let cols = [
            (MINT, l.pick("Investigar", "Research")),
            (CORAL, l.pick("Diseñar", "Design")),
            (INK, l.pick("Lanzar", "Launch")),
        ];
        for (i, (color, title)) in cols.iter().enumerate() {
            let x = 880.0 + i as f64 * 340.0;
            d.rect(p, (x, 300.0, 300.0, 480.0), CANVAS, 24.0);
            d.rect(p, (x, 300.0, 300.0, 16.0), color, 8.0);
            d.text(
                p,
                (x + 30.0, 360.0, 240.0),
                title,
                font("Manrope", 44.0, INK).bold(),
            );
            d.text(
                p,
                (x + 30.0, 440.0, 240.0),
                l.pick(
                    "Describe aquí este paso en pocas palabras.",
                    "Describe this step here in a few words.",
                ),
                font("Inter", 28.0, SLATE).line(1.4),
            );
        }
    });
    d.document()
}

fn minimal_report(l: Lang) -> Document {
    let mut d = Doc::new();
    let size = (1920.0, 1080.0, "px");
    d.page(size, solid(WHITE), |d, p| {
        d.rect(p, (1180.0, 0.0, 740.0, 1080.0), MINT_PALE, 0.0);
        d.ellipse(p, (1330.0, 290.0, 440.0, 440.0), MINT, 1.0);
        d.rect(p, (1430.0, 390.0, 240.0, 240.0), WHITE, 120.0);
        d.text(
            p,
            (150.0, 200.0, 900.0),
            l.pick("INFORME ANUAL", "ANNUAL REPORT"),
            font("Montserrat", 34.0, MINT_DARK).bold().spaced(8.0),
        );
        d.text(
            p,
            (150.0, 280.0, 950.0),
            l.pick("Resultados\nque crecen", "Results\nthat grow"),
            font("DM Serif Display", 150.0, INK).line(0.85),
        );
        d.text(
            p,
            (150.0, 650.0, 800.0),
            l.pick(
                "Un año de trabajo resumido en datos claros.",
                "A year of work summed up in clear data.",
            ),
            font("Inter", 38.0, SLATE).line(1.4),
        );
        d.text(
            p,
            (150.0, 900.0, 600.0),
            "2026",
            font("Manrope", 44.0, INK).bold(),
        );
    });
    d.page(size, solid(WHITE), |d, p| {
        d.text(
            p,
            (150.0, 140.0, 1600.0),
            l.pick("Lo más destacado", "Highlights"),
            font("DM Serif Display", 96.0, INK),
        );
        let stats = [
            ("+48%", l.pick("ventas", "sales")),
            ("12k", l.pick("clientes nuevos", "new customers")),
            ("4.9", l.pick("valoración media", "average rating")),
        ];
        for (i, (number, label)) in stats.iter().enumerate() {
            let x = 150.0 + i as f64 * 560.0;
            d.rect(p, (x, 380.0, 500.0, 420.0), MINT_PALE, 28.0);
            d.text(
                p,
                (x + 50.0, 470.0, 400.0),
                number,
                font("Manrope", 130.0, MINT_DARK).bold(),
            );
            d.text(p, (x + 50.0, 650.0, 400.0), label, font("Inter", 40.0, INK));
        }
    });
    d.document()
}

fn concert_flyer(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((210.0, 297.0, "mm"), solid(INK), |d, p| {
        let (w, h) = (p.w, p.h);
        d.ellipse(p, (w * 0.15, h * 0.08, w * 0.7, w * 0.7), CORAL, 1.0);
        d.ellipse(
            p,
            (w * 0.25, h * 0.08 + w * 0.1, w * 0.5, w * 0.5),
            MINT,
            1.0,
        );
        d.ellipse(
            p,
            (w * 0.35, h * 0.08 + w * 0.2, w * 0.3, w * 0.3),
            INK,
            1.0,
        );
        d.text(
            p,
            (50.0, h * 0.52, w - 100.0),
            l.pick("NOCHE DE JAZZ", "JAZZ NIGHT"),
            font("Bebas Neue", 120.0, WHITE).center().spaced(4.0),
        );
        d.text(
            p,
            (50.0, h * 0.64, w - 100.0),
            l.pick("Cuarteto del Sur en concierto", "The South Quartet live"),
            font("Montserrat", 30.0, MINT).center(),
        );
        d.rect(p, (w * 0.3, h * 0.72, w * 0.4, 3.0), CORAL, 1.5);
        d.text(
            p,
            (50.0, h * 0.76, w - 100.0),
            l.pick("SÁBADO 14 DE MARZO · 21:00", "SATURDAY MARCH 14 · 9 PM"),
            font("Montserrat", 26.0, WHITE).bold().center().spaced(2.0),
        );
        d.text(
            p,
            (50.0, h * 0.82, w - 100.0),
            l.pick(
                "Teatro Central · Entrada libre",
                "Central Theatre · Free entry",
            ),
            font("Montserrat", 24.0, BORDER).center(),
        );
    });
    d.document()
}

fn birthday(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((105.0, 148.0, "mm"), solid(CANVAS), |d, p| {
        let (w, h) = (p.w, p.h);
        d.ellipse(p, (-60.0, -60.0, 200.0, 200.0), CORAL, 0.35);
        d.ellipse(p, (w - 120.0, h - 140.0, 220.0, 220.0), MINT, 0.8);
        d.star(p, (w - 90.0, 40.0, 50.0, 50.0), CORAL);
        d.star(p, (40.0, h - 100.0, 40.0, 40.0), CORAL);
        d.text(
            p,
            (30.0, h * 0.14, w - 60.0),
            l.pick("Te invito a celebrar", "You're invited to celebrate"),
            font("Dancing Script", 34.0, MINT_DARK).center(),
        );
        d.text(
            p,
            (30.0, h * 0.24, w - 60.0),
            l.pick("mis", "my"),
            font("Montserrat", 22.0, INK).center().spaced(6.0),
        );
        d.text(
            p,
            (30.0, h * 0.29, w - 60.0),
            "30",
            font("Shrikhand", 140.0, CORAL).center(),
        );
        d.text(
            p,
            (30.0, h * 0.64, w - 60.0),
            "Elena",
            font("Great Vibes", 64.0, INK).center(),
        );
        d.text(
            p,
            (30.0, h * 0.8, w - 60.0),
            l.pick(
                "Sábado 13 de marzo · 20:00\nCalle Mayor 123",
                "Saturday, March 13 · 8 PM\n123 Main Street",
            ),
            font("Montserrat", 16.0, INK).center().line(1.5),
        );
    });
    d.document()
}

fn business_card(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((85.0, 55.0, "mm"), solid(WHITE), |d, p| {
        let (w, h) = (p.w, p.h);
        d.rect(p, (0.0, 0.0, w * 0.36, h), INK, 0.0);
        d.ellipse(p, (w * 0.08, h * 0.28, w * 0.2, w * 0.2), CORAL, 1.0);
        d.text(
            p,
            (w * 0.08, h * 0.35, w * 0.2),
            "AG",
            font("Manrope", 22.0, INK).bold().center(),
        );
        d.text(
            p,
            (w * 0.42, h * 0.2, w * 0.55),
            "Ana García",
            font("Manrope", 22.0, INK).bold(),
        );
        d.text(
            p,
            (w * 0.42, h * 0.36, w * 0.55),
            l.pick("Diseñadora gráfica", "Graphic designer"),
            font("Inter", 11.0, MINT_DARK),
        );
        d.text(
            p,
            (w * 0.42, h * 0.56, w * 0.55),
            "+34 600 000 000\nana@estudio.com\nestudio.com",
            font("Inter", 10.0, SLATE).line(1.5),
        );
    });
    d.document()
}

fn resume(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((210.0, 297.0, "mm"), solid(WHITE), |d, p| {
        let (w, h) = (p.w, p.h);
        d.rect(p, (0.0, 0.0, w * 0.34, h), MIST, 0.0);
        circle_photo(d, p, (w * 0.07, 60.0, w * 0.2, w * 0.2), MINT_TINT);
        d.text(p, (w * 0.04, 60.0 + w * 0.2 + 30.0, w * 0.26), l.pick("CONTACTO", "CONTACT"), font("Montserrat", 14.0, INK).bold().spaced(3.0));
        d.text(p, (w * 0.04, 60.0 + w * 0.2 + 60.0, w * 0.26), "+34 600 000 000\nnombre@correo.com\nMadrid", font("Inter", 12.0, SLATE).line(1.6));
        d.text(p, (w * 0.04, 60.0 + w * 0.2 + 170.0, w * 0.26), l.pick("HABILIDADES", "SKILLS"), font("Montserrat", 14.0, INK).bold().spaced(3.0));
        d.text(p, (w * 0.04, 60.0 + w * 0.2 + 200.0, w * 0.26), l.pick("Diseño de marca\nIlustración\nTipografía\nFotografía", "Branding\nIllustration\nTypography\nPhotography"), font("Inter", 12.0, SLATE).line(1.6));
        d.text(p, (w * 0.4, 70.0, w * 0.56), "Laura Méndez", font("Manrope", 40.0, INK).bold());
        d.text(p, (w * 0.4, 125.0, w * 0.56), l.pick("Diseñadora de producto", "Product designer"), font("Inter", 16.0, MINT_DARK));
        d.rect(p, (w * 0.4, 165.0, 60.0, 4.0), CORAL, 2.0);
        let sections = [
            (l.pick("PERFIL", "PROFILE"), l.pick("Diseño experiencias claras y útiles. Ocho años creando productos digitales con equipos pequeños.", "I design clear, useful experiences. Eight years making digital products with small teams.")),
            (l.pick("EXPERIENCIA", "EXPERIENCE"), l.pick("Estudio Norte · 2021 – hoy\nLidera el diseño de dos aplicaciones.\n\nAgencia Sur · 2017 – 2021\nIdentidades y campañas para 30 marcas.", "North Studio · 2021 – now\nLeads the design of two apps.\n\nSouth Agency · 2017 – 2021\nIdentities and campaigns for 30 brands.")),
            (l.pick("FORMACIÓN", "EDUCATION"), l.pick("Grado en Diseño · Universidad de las Artes", "BA in Design · University of the Arts")),
        ];
        let mut y = 210.0;
        for (title, body) in sections {
            d.text(p, (w * 0.4, y, w * 0.56), title, font("Montserrat", 14.0, INK).bold().spaced(3.0));
            d.text(p, (w * 0.4, y + 30.0, w * 0.54), body, font("Inter", 13.0, SLATE).line(1.55));
            y += 60.0 + body.lines().count() as f64 * 21.0 + 40.0;
        }
        let _ = h;
    });
    d.document()
}

fn newsletter(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((600.0, 200.0, "px"), solid(MINT_PALE), |d, p| {
        d.ellipse(p, (430.0, -60.0, 220.0, 220.0), MINT, 1.0);
        d.ellipse(p, (500.0, 110.0, 120.0, 120.0), CORAL, 0.9);
        d.text(
            p,
            (36.0, 50.0, 400.0),
            l.pick("Boletín de noviembre", "November newsletter"),
            font("Manrope", 34.0, INK).bold(),
        );
        d.text(
            p,
            (36.0, 110.0, 380.0),
            l.pick(
                "Novedades, consejos y lo que viene.",
                "News, tips and what's coming.",
            ),
            font("Inter", 17.0, SLATE),
        );
    });
    d.document()
}

fn web_banner(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(
        (1200.0, 300.0, "px"),
        json!({ "gradient": linear(0.0, INK, INK_LIGHT) }),
        |d, p| {
            d.ellipse(p, (900.0, -120.0, 360.0, 360.0), CORAL, 1.0);
            d.ellipse(p, (1040.0, 150.0, 240.0, 240.0), MINT, 1.0);
            d.text(
                p,
                (70.0, 70.0, 760.0),
                l.pick("Lanza tu tienda en línea", "Launch your online store"),
                font("Poppins", 52.0, WHITE).bold(),
            );
            d.text(
                p,
                (70.0, 150.0, 760.0),
                l.pick(
                    "Plantillas listas para vender desde hoy.",
                    "Templates ready to sell from today.",
                ),
                font("Inter", 24.0, BORDER),
            );
            d.rect(p, (70.0, 205.0, 210.0, 54.0), CORAL, 27.0);
            d.text(
                p,
                (70.0, 218.0, 210.0),
                l.pick("Empezar", "Get started"),
                font("Poppins", 22.0, INK).bold().center(),
            );
        },
    );
    d.document()
}

fn certificate(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((297.0, 210.0, "mm"), solid(CANVAS), |d, p| {
        let (w, h) = (p.w, p.h);
        d.frame(p, (30.0, 30.0, w - 60.0, h - 60.0), MINT_DARK, 6.0, 0.0);
        d.frame(p, (46.0, 46.0, w - 92.0, h - 92.0), MINT_DARK, 1.5, 0.0);
        d.text(
            p,
            (80.0, h * 0.16, w - 160.0),
            l.pick("CERTIFICADO", "CERTIFICATE"),
            font("Cormorant Garamond", 72.0, INK)
                .bold()
                .center()
                .spaced(10.0),
        );
        d.text(
            p,
            (80.0, h * 0.3, w - 160.0),
            l.pick("DE RECONOCIMIENTO", "OF RECOGNITION"),
            font("Montserrat", 20.0, MINT_DARK).center().spaced(8.0),
        );
        d.text(
            p,
            (80.0, h * 0.4, w - 160.0),
            l.pick("Se otorga a", "Presented to"),
            font("Montserrat", 20.0, SLATE).center(),
        );
        d.text(
            p,
            (80.0, h * 0.47, w - 160.0),
            "Daniel Ortega",
            font("Great Vibes", 84.0, INK).center(),
        );
        d.rect(p, (w * 0.3, h * 0.64, w * 0.4, 2.0), MINT_DARK, 1.0);
        d.text(
            p,
            (120.0, h * 0.68, w - 240.0),
            l.pick(
                "Por su compromiso y su excelente trabajo durante el año.",
                "For outstanding commitment and work throughout the year.",
            ),
            font("Montserrat", 18.0, SLATE).center(),
        );
        d.ellipse(p, (w / 2.0 - 45.0, h * 0.77, 90.0, 90.0), MINT_DARK, 1.0);
        d.star(p, (w / 2.0 - 25.0, h * 0.77 + 20.0, 50.0, 50.0), CANVAS);
    });
    d.document()
}

fn tips_carousel(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((1080.0, 1080.0, "px"), solid(INK), |d, p| {
        d.rect(p, (60.0, 60.0, 960.0, 960.0), INK_LIGHT, 40.0);
        d.ellipse(p, (720.0, 120.0, 220.0, 220.0), CORAL, 1.0);
        d.text(
            p,
            (720.0, 170.0, 220.0),
            "3",
            font("Manrope", 110.0, INK).bold().center(),
        );
        d.text(
            p,
            (130.0, 380.0, 820.0),
            l.pick(
                "consejos para\ncolores que\nfuncionan",
                "tips for\ncolours that\nwork",
            ),
            font("Manrope", 92.0, WHITE).bold().line(1.05),
        );
        d.text(
            p,
            (130.0, 820.0, 820.0),
            l.pick("Desliza →", "Swipe →"),
            font("Inter", 36.0, MINT),
        );
    });
    d.document()
}

fn product_post(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page((1200.0, 627.0, "px"), solid(CANVAS), |d, p| {
        d.rect(p, (640.0, 40.0, 520.0, 547.0), MINT_PALE, 32.0);
        d.ellipse(p, (720.0, 120.0, 360.0, 360.0), MINT, 1.0);
        d.rect(p, (820.0, 200.0, 160.0, 220.0), WHITE, 20.0);
        d.text(
            p,
            (70.0, 110.0, 540.0),
            l.pick("Contratamos", "We're hiring"),
            font("Manrope", 76.0, INK).bold(),
        );
        d.text(
            p,
            (70.0, 220.0, 520.0),
            l.pick(
                "Buscamos una persona diseñadora para unirse al equipo.",
                "We're looking for a designer to join the team.",
            ),
            font("Inter", 28.0, SLATE).line(1.4),
        );
        d.rect(p, (70.0, 400.0, 270.0, 66.0), INK, 33.0);
        d.text(
            p,
            (70.0, 417.0, 270.0),
            l.pick("Postúlate", "Apply now"),
            font("Inter", 26.0, WHITE).bold().center(),
        );
    });
    d.document()
}

// ------------------------------------------------- presentations (16:9)

const SLIDE: (f64, f64, &str) = (1920.0, 1080.0, "px");

fn brandbook(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SLIDE, pattern("stripes", MINT_TINT, CANVAS), |d, p| {
        d.rect(p, (160.0, 200.0, 1600.0, 680.0), CANVAS, 0.0);
        d.text(
            p,
            (1500.0, 240.0, 220.0),
            "Vol.02",
            font("League Spartan", 44.0, MINT_DARK).right(),
        );
        d.text(
            p,
            (160.0, 330.0, 1600.0),
            "Brandbook",
            font("Archivo Black", 230.0, INK).center().line(0.9),
        );
        d.text(
            p,
            (160.0, 640.0, 1600.0),
            l.pick(
                "Identidad  ·  Esencia  ·  Sistema visual",
                "Identity  ·  Essence  ·  Visual system",
            ),
            font("Questrial", 52.0, MINT_DARK).center().spaced(2.0),
        );
        d.text(
            p,
            (220.0, 800.0, 700.0),
            "www.tumarca.com",
            font("Inter", 24.0, SLATE),
        );
        d.text(
            p,
            (1000.0, 800.0, 700.0),
            l.pick("Septiembre 2026", "September 2026"),
            font("Inter", 24.0, SLATE).right(),
        );
    });
    d.page(SLIDE, solid(CANVAS), |d, p| {
        d.text(
            p,
            (160.0, 130.0, 1600.0),
            l.pick("Paleta de color", "Colour palette"),
            font("Archivo Black", 90.0, INK),
        );
        let swatches = [
            (INK, "Tinta", "Ink"),
            (CORAL, "Coral", "Coral"),
            (MINT, "Menta", "Mint"),
            (MIST, "Bruma", "Mist"),
            (CANVAS, "Lienzo", "Canvas"),
        ];
        for (i, (color, es, en)) in swatches.iter().enumerate() {
            let x = 160.0 + i as f64 * 330.0;
            d.rect(p, (x, 340.0, 290.0, 420.0), color, 24.0);
            d.frame(p, (x, 340.0, 290.0, 420.0), BORDER, 2.0, 24.0);
            d.text(
                p,
                (x, 800.0, 290.0),
                l.pick(es, en),
                font("Manrope", 40.0, INK).bold(),
            );
            d.text(p, (x, 870.0, 290.0), color, font("Inter", 28.0, SLATE));
        }
    });
    d.document()
}

fn brand_manual(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SLIDE, solid(WHITE), |d, p| {
        d.text(
            p,
            (120.0, 110.0, 400.0),
            l.pick("AÑO 2026", "YEAR 2026"),
            font("Inter", 22.0, INK).spaced(4.0),
        );
        d.rect(p, (360.0, 124.0, 1440.0, 3.0), INK, 0.0);
        d.text(
            p,
            (300.0, 330.0, 1320.0),
            l.pick("Manual de\nmarca", "Brand\nmanual"),
            font("Archivo Black", 200.0, INK).center().line(0.82),
        );
        d.burst(p, (1340.0, 520.0, 170.0, 170.0), CORAL, 18, 0.18);
        d.rect(p, (120.0, 950.0, 1300.0, 3.0), INK, 0.0);
        d.text(
            p,
            (1440.0, 936.0, 360.0),
            "www.tumarca.com",
            font("Inter", 22.0, INK).right().spaced(2.0),
        );
    });
    d.page(SLIDE, solid(WHITE), |d, p| {
        d.text(
            p,
            (120.0, 120.0, 1500.0),
            l.pick("Nuestros valores", "Our values"),
            font("Archivo Black", 96.0, INK),
        );
        d.burst(p, (1640.0, 110.0, 140.0, 140.0), CORAL, 18, 0.18);
        let values = [
            (
                l.pick("Claridad", "Clarity"),
                l.pick(
                    "Decimos lo justo, con palabras sencillas.",
                    "We say just enough, in plain words.",
                ),
            ),
            (
                l.pick("Cercanía", "Warmth"),
                l.pick(
                    "Hablamos como hablaría una persona amiga.",
                    "We talk the way a friend would.",
                ),
            ),
            (
                l.pick("Constancia", "Consistency"),
                l.pick(
                    "Cada pieza se reconoce como nuestra.",
                    "Every piece is recognisably ours.",
                ),
            ),
        ];
        for (i, (title, body)) in values.iter().enumerate() {
            let x = 120.0 + i as f64 * 580.0;
            d.text(
                p,
                (x, 380.0, 520.0),
                &format!("0{}", i + 1),
                font("Archivo Black", 120.0, CORAL),
            );
            d.text(
                p,
                (x, 600.0, 520.0),
                title,
                font("Manrope", 52.0, INK).bold(),
            );
            d.text(
                p,
                (x, 690.0, 480.0),
                body,
                font("Inter", 32.0, SLATE).line(1.4),
            );
        }
    });
    d.document()
}

fn marketing_plan(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(
        SLIDE,
        json!({ "gradient": linear(30.0, INK, CORAL) }),
        |d, p| {
            d.ellipse(p, (1300.0, -300.0, 900.0, 900.0), CORAL_LIGHT, 0.35);
            d.text(
                p,
                (160.0, 250.0, 1600.0),
                l.pick("ESTRATEGIAS DE REDES SOCIALES", "SOCIAL MEDIA STRATEGY"),
                font("Montserrat", 30.0, CANVAS).bold().center().spaced(6.0),
            );
            d.text(
                p,
                (160.0, 360.0, 1600.0),
                l.pick("PLAN DE MARKETING", "MARKETING PLAN"),
                font("Bebas Neue", 200.0, WHITE).center().spaced(6.0),
            );
            d.text(
                p,
                (160.0, 640.0, 1600.0),
                l.pick("OCTUBRE 2026", "OCTOBER 2026"),
                font("Montserrat", 36.0, WHITE).bold().center().spaced(4.0),
            );
            d.rect(p, (160.0, 920.0, 1600.0, 2.0), CANVAS, 0.0);
            d.text(
                p,
                (160.0, 950.0, 600.0),
                "@tumarca",
                font("Inter", 24.0, CANVAS),
            );
            d.text(
                p,
                (1160.0, 950.0, 600.0),
                "www.tumarca.com",
                font("Inter", 24.0, CANVAS).right(),
            );
        },
    );
    d.page(SLIDE, solid(CANVAS), |d, p| {
        d.text(
            p,
            (160.0, 120.0, 1600.0),
            l.pick("CALENDARIO", "TIMELINE"),
            font("Bebas Neue", 120.0, INK).spaced(4.0),
        );
        d.rect(p, (160.0, 560.0, 1600.0, 6.0), BORDER, 3.0);
        let steps = [
            (
                "T1",
                l.pick("Investigar a la audiencia", "Research the audience"),
            ),
            ("T2", l.pick("Crear el contenido", "Create the content")),
            ("T3", l.pick("Lanzar la campaña", "Launch the campaign")),
            ("T4", l.pick("Medir y ajustar", "Measure and adjust")),
        ];
        for (i, (quarter, text)) in steps.iter().enumerate() {
            let x = 160.0 + i as f64 * 420.0;
            d.ellipse(
                p,
                (x, 503.0, 120.0, 120.0),
                if i % 2 == 0 { CORAL } else { INK },
                1.0,
            );
            d.text(
                p,
                (x, 528.0, 120.0),
                quarter,
                font("Bebas Neue", 60.0, WHITE).center(),
            );
            d.text(
                p,
                (x, 680.0, 360.0),
                text,
                font("Inter", 34.0, INK).line(1.3),
            );
        }
    });
    d.document()
}

fn color_psychology(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SLIDE, solid(CANVAS), |d, p| {
        let bands = [
            CORAL,
            CORAL_LIGHT,
            "#FFC9BF",
            MINT_TINT,
            MINT,
            MINT_DARK,
            INK_LIGHT,
            INK,
        ];
        for (i, color) in bands.iter().enumerate() {
            d.rect(p, (i as f64 * 240.0, 0.0, 240.0, 600.0), color, 0.0);
        }
        d.text(
            p,
            (160.0, 700.0, 1600.0),
            l.pick("PSICOLOGÍA\nDEL COLOR", "THE PSYCHOLOGY\nOF COLOUR"),
            font("Chau Philomene One", 96.0, INK)
                .center()
                .spaced(12.0)
                .line(0.95),
        );
    });
    d.page(SLIDE, solid(CANVAS), |d, p| {
        d.text(
            p,
            (160.0, 120.0, 1600.0),
            l.pick("Lo que transmite cada color", "What each colour says"),
            font("Chau Philomene One", 80.0, INK).spaced(4.0),
        );
        let colors = [
            (
                CORAL,
                l.pick("Energía", "Energy"),
                l.pick(
                    "Llama la atención e invita a actuar.",
                    "Grabs attention and invites action.",
                ),
            ),
            (
                MINT,
                l.pick("Calma", "Calm"),
                l.pick(
                    "Transmite frescura, equilibrio y confianza.",
                    "Feels fresh, balanced and trustworthy.",
                ),
            ),
            (
                INK,
                l.pick("Solidez", "Strength"),
                l.pick(
                    "Da seriedad y hace legible el texto.",
                    "Brings weight and keeps text legible.",
                ),
            ),
        ];
        for (i, (color, title, body)) in colors.iter().enumerate() {
            let x = 160.0 + i as f64 * 560.0;
            d.ellipse(p, (x, 340.0, 200.0, 200.0), color, 1.0);
            d.text(
                p,
                (x, 600.0, 480.0),
                title,
                font("Manrope", 52.0, INK).bold(),
            );
            d.text(
                p,
                (x, 690.0, 460.0),
                body,
                font("Inter", 32.0, SLATE).line(1.4),
            );
        }
    });
    d.document()
}

fn creative_brief(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SLIDE, solid(CANVAS), |d, p| {
        d.text(
            p,
            (100.0, 80.0, 400.0),
            l.pick("ESTUDIO\nNORTE", "NORTH\nSTUDIO"),
            font("Archivo Black", 22.0, INK).line(0.9),
        );
        d.text(
            p,
            (760.0, 80.0, 500.0),
            l.pick(
                "DIRECCIÓN ESTRATÉGICA\nY CONCEPTUAL",
                "STRATEGIC AND\nCONCEPT DIRECTION",
            ),
            font("Archivo Black", 22.0, INK).line(0.9),
        );
        d.text(
            p,
            (1520.0, 80.0, 300.0),
            l.pick("JULIO\n2026", "JULY\n2026"),
            font("Archivo Black", 22.0, INK).right().line(0.9),
        );
        d.text(
            p,
            (100.0, 380.0, 1000.0),
            "BRIEF",
            font("Archivo Black", 230.0, INK),
        );
        d.burst(p, (900.0, 380.0, 240.0, 240.0), CORAL, 6, 0.12);
        d.rect(p, (100.0, 680.0, 1400.0, 290.0), MINT_TINT, 0.0);
        d.text(
            p,
            (120.0, 660.0, 1500.0),
            l.pick("CREATIVO", "CREATIVE"),
            font("Archivo Black", 230.0, INK),
        );
    });
    d.page(SLIDE, solid(CANVAS), |d, p| {
        d.text(
            p,
            (100.0, 110.0, 1700.0),
            l.pick(
                "El encargo, en tres preguntas",
                "The brief in three questions",
            ),
            font("Archivo Black", 80.0, INK),
        );
        let boxes = [
            (
                l.pick("¿Qué buscamos?", "What do we want?"),
                l.pick(
                    "El objetivo de la campaña, en una frase.",
                    "The aim of the campaign, in one sentence.",
                ),
            ),
            (
                l.pick("¿Para quién?", "For whom?"),
                l.pick(
                    "La persona a la que le hablamos.",
                    "The person we are talking to.",
                ),
            ),
            (
                l.pick("¿Qué decimos?", "What do we say?"),
                l.pick(
                    "El mensaje que debe recordar.",
                    "The message they should remember.",
                ),
            ),
        ];
        for (i, (title, body)) in boxes.iter().enumerate() {
            let x = 100.0 + i as f64 * 590.0;
            d.rect(
                p,
                (x, 340.0, 540.0, 560.0),
                if i == 1 { MINT_TINT } else { WHITE },
                0.0,
            );
            d.frame(p, (x, 340.0, 540.0, 560.0), INK, 4.0, 0.0);
            d.burst(p, (x + 40.0, 380.0, 80.0, 80.0), CORAL, 6, 0.12);
            d.text(
                p,
                (x + 40.0, 500.0, 460.0),
                title,
                font("Archivo Black", 48.0, INK).line(1.0),
            );
            d.text(
                p,
                (x + 40.0, 640.0, 460.0),
                body,
                font("Inter", 32.0, SLATE).line(1.4),
            );
        }
    });
    d.document()
}

fn final_project(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SLIDE, solid(INK), |d, p| {
        d.frame(p, (120.0, 680.0, 260.0, 260.0), CORAL, 10.0, 0.0);
        Doc::turn_last(p, 20.0);
        d.polygon(
            p,
            (1500.0, 80.0, 300.0, 260.0),
            MINT,
            &[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)],
            0.25,
        );
        d.ring(p, (1450.0, 650.0, 600.0, 600.0), MINT, 10.0);
        d.ring(p, (-160.0, -200.0, 520.0, 520.0), CORAL, 8.0);
        d.text(
            p,
            (160.0, 400.0, 1600.0),
            l.pick("Proyecto Final", "Final Project"),
            font("Poppins", 150.0, CORAL).bold().center(),
        );
        d.rect(p, (460.0, 650.0, 1000.0, 10.0), MINT, 5.0);
        d.text(
            p,
            (160.0, 720.0, 1600.0),
            l.pick("Curso de diseño · Grupo 3", "Design course · Group 3"),
            font("Inter", 36.0, MIST).center(),
        );
    });
    d.page(SLIDE, solid(INK), |d, p| {
        d.text(
            p,
            (160.0, 120.0, 1600.0),
            l.pick("El equipo", "The team"),
            font("Poppins", 96.0, CORAL).bold(),
        );
        let people = [
            ("Ana", CORAL),
            ("Luis", MINT),
            ("Sara", CORAL_LIGHT),
            ("Pablo", MINT_TINT),
        ];
        for (i, (name, color)) in people.iter().enumerate() {
            let x = 160.0 + i as f64 * 420.0;
            circle_photo(d, p, (x, 340.0, 300.0, 300.0), color);
            d.text(
                p,
                (x, 690.0, 300.0),
                name,
                font("Poppins", 44.0, WHITE).bold().center(),
            );
            d.text(
                p,
                (x, 760.0, 300.0),
                l.pick("Rol en el proyecto", "Role in the project"),
                font("Inter", 26.0, MIST).center(),
            );
        }
    });
    d.document()
}

fn web_design(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SLIDE, solid(CORAL_LIGHT), |d, p| {
        d.ellipse(p, (-200.0, 600.0, 700.0, 700.0), CORAL, 0.6);
        d.ellipse(p, (1500.0, -250.0, 700.0, 700.0), MINT_TINT, 1.0);
        d.rect(p, (0.0, 300.0, 1250.0, 500.0), INK, 0.0);
        d.text(
            p,
            (100.0, 340.0, 1100.0),
            l.pick("F U N D A M E N T O S", "F U N D A M E N T A L S"),
            font("Montserrat", 30.0, WHITE).bold(),
        );
        d.text(
            p,
            (100.0, 410.0, 700.0),
            l.pick("DISEÑO\nWEB", "WEB\nDESIGN"),
            font("Archivo Black", 130.0, WHITE).line(0.85),
        );
        d.rect(p, (790.0, 450.0, 6.0, 260.0), CORAL, 0.0);
        d.text(
            p,
            (830.0, 480.0, 380.0),
            l.pick(
                "Por:\nSandra Haro y\nFrancisco Mercado",
                "By:\nSandra Haro and\nFrancisco Mercado",
            ),
            font("Inter", 30.0, WHITE).line(1.3),
        );
    });
    d.page(SLIDE, solid(CANVAS), |d, p| {
        d.rect(p, (0.0, 0.0, 1920.0, 220.0), INK, 0.0);
        d.text(
            p,
            (100.0, 60.0, 1700.0),
            l.pick("Tres principios", "Three principles"),
            font("Archivo Black", 80.0, WHITE),
        );
        let items = [
            (
                l.pick("Jerarquía", "Hierarchy"),
                l.pick(
                    "Lo importante se ve primero.",
                    "What matters is seen first.",
                ),
            ),
            (
                l.pick("Contraste", "Contrast"),
                l.pick(
                    "El texto se lee sin esfuerzo.",
                    "Text reads without effort.",
                ),
            ),
            (
                l.pick("Ritmo", "Rhythm"),
                l.pick(
                    "Espacios iguales, página tranquila.",
                    "Even spacing, a calm page.",
                ),
            ),
        ];
        for (i, (title, body)) in items.iter().enumerate() {
            let x = 100.0 + i as f64 * 590.0;
            d.rect(p, (x, 340.0, 540.0, 600.0), WHITE, 24.0);
            d.rect(p, (x + 40.0, 390.0, 460.0, 220.0), CORAL_LIGHT, 16.0);
            d.rect(p, (x + 70.0, 420.0, 200.0, 24.0), WHITE, 12.0);
            d.rect(p, (x + 70.0, 470.0, 300.0, 24.0), WHITE, 12.0);
            d.text(
                p,
                (x + 40.0, 660.0, 460.0),
                title,
                font("Archivo Black", 48.0, INK),
            );
            d.text(
                p,
                (x + 40.0, 760.0, 460.0),
                body,
                font("Inter", 32.0, SLATE).line(1.4),
            );
        }
    });
    d.document()
}

fn portfolio(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SLIDE, solid(WHITE), |d, p| {
        let tones = [
            MINT_TINT,
            CORAL_LIGHT,
            MIST,
            MINT,
            INK_LIGHT,
            "#FFC9BF",
            MINT_PALE,
            CORAL,
        ];
        let (w, h) = (480.0, 270.0);
        let mut i = 0;
        for row in 0..4 {
            for col in 0..4 {
                if (1..=2).contains(&row) && (1..=2).contains(&col) {
                    continue;
                }
                photo(
                    d,
                    p,
                    (col as f64 * w + 4.0, row as f64 * h + 4.0, w - 8.0, h - 8.0),
                    tones[i % tones.len()],
                );
                i += 1;
            }
        }
        d.text(
            p,
            (480.0, 400.0, 960.0),
            "Cristian León",
            font("Cormorant Garamond", 110.0, INK).center(),
        );
        d.frame(p, (700.0, 600.0, 520.0, 64.0), INK, 2.0, 0.0);
        d.text(
            p,
            (700.0, 612.0, 520.0),
            l.pick("PORTAFOLIO PROFESIONAL", "PROFESSIONAL PORTFOLIO"),
            font("Montserrat", 26.0, INK).center().spaced(4.0),
        );
    });
    d.page(SLIDE, solid(WHITE), |d, p| {
        d.text(
            p,
            (120.0, 110.0, 1700.0),
            l.pick("Proyectos destacados", "Selected work"),
            font("Cormorant Garamond", 96.0, INK),
        );
        let tones = [MINT_TINT, CORAL_LIGHT, MIST, MINT, INK_LIGHT, MINT_PALE];
        for (i, tone) in tones.iter().enumerate() {
            let (col, row) = (i % 3, i / 3);
            let (x, y) = (120.0 + col as f64 * 570.0, 300.0 + row as f64 * 360.0);
            photo(d, p, (x, y, 530.0, 250.0), tone);
            d.text(
                p,
                (x, y + 265.0, 530.0),
                &format!("{} 0{}", l.pick("Proyecto", "Project"), i + 1),
                font("Montserrat", 26.0, INK).spaced(2.0),
            );
        }
    });
    d.document()
}

// ----------------------------------------------------- trifold brochures

/// A trifold, 11 × 8.5 in: three panels of 352 px a side.
const TRIFOLD: (f64, f64, &str) = (11.0, 8.5, "in");
const PANEL: f64 = 352.0;

/// The left edge of panel `i` plus the inner margin.
fn panel(i: u32) -> f64 {
    f64::from(i) * PANEL + 32.0
}

const PANEL_W: f64 = PANEL - 64.0;

fn beauty_center(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(TRIFOLD, solid(CANVAS), |d, p| {
        d.rect(p, (PANEL, 0.0, PANEL, 816.0), MINT_DARK, 0.0);
        d.text(
            p,
            (panel(0), 60.0, PANEL_W),
            l.pick("Masajes\nmodeladores", "Shaping\nmassages"),
            font("Cormorant Garamond", 30.0, INK).line(0.9),
        );
        d.text(
            p,
            (panel(0), 150.0, PANEL_W),
            l.pick(
                "Técnicas manuales que relajan, tonifican y devuelven la energía.",
                "Hand techniques that relax, tone and restore energy.",
            ),
            font("Montserrat", 13.0, SLATE).line(1.5),
        );
        d.rect(p, (panel(0), 230.0, 180.0, 26.0), MINT_DARK, 0.0);
        d.text(
            p,
            (panel(0) + 8.0, 234.0, 170.0),
            l.pick("SESIÓN DE 60 MIN", "60-MIN SESSION"),
            font("Montserrat", 11.0, WHITE).bold().spaced(2.0),
        );
        d.text(
            p,
            (panel(0), 300.0, PANEL_W),
            l.pick("Limpieza facial", "Facial cleanse"),
            font("Cormorant Garamond", 30.0, INK),
        );
        d.text(
            p,
            (panel(0), 350.0, PANEL_W),
            l.pick(
                "Piel luminosa y suave con productos naturales.",
                "Bright, soft skin with natural products.",
            ),
            font("Montserrat", 13.0, SLATE).line(1.5),
        );
        d.text(
            p,
            (panel(0), 470.0, PANEL_W),
            l.pick("Lifting de pestañas", "Lash lift"),
            font("Cormorant Garamond", 30.0, INK),
        );
        d.text(
            p,
            (panel(0), 520.0, PANEL_W),
            l.pick(
                "Una mirada abierta durante semanas.",
                "An open look that lasts for weeks.",
            ),
            font("Montserrat", 13.0, SLATE).line(1.5),
        );
        d.text(
            p,
            (panel(1), 300.0, PANEL_W),
            l.pick("CONTÁCTANOS", "CONTACT US"),
            font("Cormorant Garamond", 30.0, WHITE).center().spaced(4.0),
        );
        d.text(
            p,
            (panel(1), 380.0, PANEL_W),
            "+34 600 000 000\nhola@centro.com\nCalle Mayor 12",
            font("Montserrat", 15.0, WHITE).center().line(1.8),
        );
        circle_photo(d, p, (2.0 * PANEL + 66.0, 150.0, 220.0, 220.0), MINT_TINT);
        d.text(
            p,
            (panel(2), 480.0, PANEL_W),
            l.pick("CENTRO DE\nESTÉTICA", "BEAUTY\nCENTRE"),
            font("Cormorant Garamond", 50.0, INK).center().line(0.95),
        );
        d.text(
            p,
            (panel(2), 640.0, PANEL_W),
            l.pick("Servicios para cada necesidad", "Treatments for every need"),
            font("Montserrat", 13.0, MINT_DARK).center(),
        );
    });
    d.page(TRIFOLD, solid(WHITE), |d, p| {
        let services = [
            (
                l.pick("Faciales", "Facials"),
                l.pick(
                    "Limpieza, hidratación y tratamientos antiedad adaptados a tu piel.",
                    "Cleansing, hydration and anti-ageing care for your skin.",
                ),
            ),
            (
                l.pick("Corporales", "Body"),
                l.pick(
                    "Masajes, exfoliación y envolturas con aceites esenciales.",
                    "Massages, scrubs and wraps with essential oils.",
                ),
            ),
            (
                l.pick("Manos y pies", "Hands and feet"),
                l.pick(
                    "Manicura y pedicura con esmaltes de larga duración.",
                    "Manicure and pedicure with long-lasting polish.",
                ),
            ),
        ];
        for (i, (title, body)) in services.iter().enumerate() {
            let x = panel(i as u32);
            photo(
                d,
                p,
                (x, 60.0, PANEL_W, 260.0),
                if i == 1 { MINT_TINT } else { MINT_PALE },
            );
            d.text(
                p,
                (x, 370.0, PANEL_W),
                title,
                font("Cormorant Garamond", 40.0, INK).center(),
            );
            d.rect(
                p,
                (x + PANEL_W / 2.0 - 30.0, 440.0, 60.0, 3.0),
                MINT_DARK,
                0.0,
            );
            d.text(
                p,
                (x, 470.0, PANEL_W),
                body,
                font("Montserrat", 15.0, SLATE).center().line(1.6),
            );
        }
    });
    d.document()
}

fn plant_nursery(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(TRIFOLD, solid(MINT), |d, p| {
        d.rect(p, (PANEL, 0.0, PANEL, 816.0), CORAL_LIGHT, 0.0);
        d.text(p, (panel(0), 120.0, PANEL_W), l.pick("DESCUBRE LA MAGIA DE LA NATURALEZA EN NUESTRA TIENDA DE SUCULENTAS.", "DISCOVER THE MAGIC OF NATURE IN OUR SUCCULENT SHOP."), font("Chau Philomene One", 22.0, WHITE).line(1.3).spaced(2.0));
        d.text(p, (panel(0), 360.0, PANEL_W), l.pick("Plantas fáciles de cuidar, macetas hechas a mano y talleres para empezar tu jardín.", "Easy-care plants, handmade pots and workshops to start your garden."), font("Montserrat", 14.0, INK).line(1.6));
        for i in 0..3 {
            d.ellipse(p, (panel(0) + f64::from(i) * 60.0, 640.0, 40.0, 40.0), WHITE, 0.7);
        }
        d.text(p, (panel(1), 300.0, PANEL_W), l.pick("NUESTRO CONTACTO", "GET IN TOUCH"), font("Chau Philomene One", 26.0, INK).center().spaced(3.0));
        d.text(p, (panel(1), 360.0, PANEL_W), "+34 600 000 000\nhola@vivero.com\nvivero.com", font("Montserrat", 15.0, INK).center().line(1.8));
        let x = 2.0 * PANEL;
        d.polygon(p, (x + 120.0, 140.0, 110.0, 200.0), MINT_DARK, &[(0.5, 0.0), (1.0, 0.6), (0.5, 1.0), (0.0, 0.6)], 1.0);
        d.polygon(p, (x + 60.0, 230.0, 110.0, 160.0), MINT_DARK, &[(0.2, 0.0), (1.0, 0.7), (0.6, 1.0), (0.0, 0.5)], 1.0);
        d.polygon(p, (x + 180.0, 230.0, 110.0, 160.0), MINT_DARK, &[(0.8, 0.0), (1.0, 0.5), (0.4, 1.0), (0.0, 0.7)], 1.0);
        d.rect(p, (x + 110.0, 380.0, 130.0, 110.0), CORAL, 12.0);
        d.text(p, (panel(2), 540.0, PANEL_W), l.pick("Vivero\nSol y Hoja", "Sun & Leaf\nNursery"), font("Pacifico", 44.0, WHITE).center().line(0.95));
        d.text(p, (panel(2), 700.0, PANEL_W), l.pick("Tienda de plantas suculentas", "Succulent plant shop"), font("Montserrat", 14.0, INK).center());
    });
    d.page(TRIFOLD, pattern("dots", MINT_TINT, CANVAS), |d, p| {
        let sections = [
            (
                l.pick("Suculentas", "Succulents"),
                l.pick(
                    "Más de 80 especies que piden poca agua y mucha luz.",
                    "Over 80 species that need little water and lots of light.",
                ),
            ),
            (
                l.pick("Macetas", "Pots"),
                l.pick(
                    "Barro, cerámica y cemento, hechas por artesanos locales.",
                    "Clay, ceramic and concrete, made by local makers.",
                ),
            ),
            (
                l.pick("Talleres", "Workshops"),
                l.pick(
                    "Aprende a trasplantar y a crear tu terrario cada sábado.",
                    "Learn to repot and build a terrarium every Saturday.",
                ),
            ),
        ];
        for (i, (title, body)) in sections.iter().enumerate() {
            let x = panel(i as u32);
            d.rect(p, (x, 70.0, PANEL_W, 680.0), WHITE, 24.0);
            circle_photo(
                d,
                p,
                (x + PANEL_W / 2.0 - 70.0, 120.0, 140.0, 140.0),
                if i == 1 { CORAL_LIGHT } else { MINT_TINT },
            );
            d.text(
                p,
                (x + 20.0, 310.0, PANEL_W - 40.0),
                title,
                font("Pacifico", 38.0, MINT_DARK).center(),
            );
            d.text(
                p,
                (x + 24.0, 400.0, PANEL_W - 48.0),
                body,
                font("Montserrat", 15.0, SLATE).center().line(1.6),
            );
        }
    });
    d.document()
}

fn interior_studio(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(TRIFOLD, solid(MIST), |d, p| {
        d.rect(p, (0.0, 0.0, PANEL, 816.0), INK_LIGHT, 0.0);
        d.rect(p, (40.0, 60.0, 120.0, 696.0), INK, 0.0);
        d.rect(p, (190.0, 60.0, 120.0, 696.0), INK, 0.0);
        d.text(
            p,
            (panel(0), 330.0, PANEL_W),
            l.pick(
                "¡CONTÁCTANOS\nY PIDE TU\nPRESUPUESTO!",
                "CONTACT US\nFOR A\nQUOTE!",
            ),
            font("Montserrat", 22.0, WHITE).bold().center().line(1.3),
        );
        d.rect(p, (PANEL, 0.0, PANEL, 816.0), MINT_DARK, 0.0);
        d.text(
            p,
            (panel(1), 260.0, PANEL_W),
            l.pick("CONTACTO", "CONTACT"),
            font("Cormorant Garamond", 34.0, WHITE).center().spaced(4.0),
        );
        d.text(
            p,
            (panel(1), 340.0, PANEL_W),
            "+34 600 000 000\nhola@estudio.com\nestudio.com",
            font("Montserrat", 15.0, WHITE).center().line(1.9),
        );
        d.ring(p, (2.0 * PANEL + 106.0, 90.0, 140.0, 140.0), INK, 2.0);
        d.text(
            p,
            (panel(2), 260.0, PANEL_W),
            l.pick("ESTUDIO\nSHONOS", "SHONOS\nSTUDIO"),
            font("Cormorant Garamond", 56.0, INK).center().line(0.95),
        );
        d.text(
            p,
            (panel(2), 420.0, PANEL_W),
            l.pick("DISEÑO DE INTERIORES", "INTERIOR DESIGN"),
            font("Montserrat", 13.0, INK).center().spaced(4.0),
        );
        rounded_photo(
            d,
            p,
            (2.0 * PANEL + 40.0, 500.0, PANEL - 80.0, 420.0),
            INK_LIGHT,
            136.0,
        );
    });
    d.page(TRIFOLD, solid(WHITE), |d, p| {
        d.text(p, (panel(0), 70.0, PANEL_W), l.pick("Quiénes\nsomos", "Who\nwe are"), font("Cormorant Garamond", 52.0, INK).line(0.95));
        d.text(p, (panel(0), 230.0, PANEL_W), l.pick("Un equipo de arquitectas y decoradoras que convierte casas en hogares, con calma y a medida.", "A team of architects and decorators who turn houses into homes, calmly and made to measure."), font("Montserrat", 15.0, SLATE).line(1.7));
        photo(d, p, (panel(0), 520.0, PANEL_W, 230.0), MIST);
        d.rect(p, (PANEL, 0.0, PANEL, 816.0), MINT_PALE, 0.0);
        d.text(p, (panel(1), 70.0, PANEL_W), l.pick("Servicios", "Services"), font("Cormorant Garamond", 52.0, INK));
        let services = [l.pick("Proyecto integral", "Full project"), l.pick("Asesoría de color", "Colour advice"), l.pick("Mobiliario a medida", "Custom furniture"), l.pick("Reformas", "Renovations")];
        for (i, s) in services.iter().enumerate() {
            let y = 200.0 + i as f64 * 110.0;
            d.ellipse(p, (panel(1), y, 18.0, 18.0), MINT_DARK, 1.0);
            d.text(p, (panel(1) + 36.0, y - 6.0, PANEL_W - 36.0), s, font("Montserrat", 18.0, INK).bold());
        }
        photo(d, p, (panel(2), 60.0, PANEL_W, 340.0), INK_LIGHT);
        d.text(p, (panel(2), 440.0, PANEL_W), l.pick("Cada espacio cuenta una historia: la tuya.", "Every space tells a story: yours."), font("Cormorant Garamond", 32.0, INK).italic().line(1.1));
    });
    d.document()
}

fn stationery(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(TRIFOLD, solid(CANVAS), |d, p| {
        d.rect(p, (0.0, 0.0, PANEL, 816.0), MINT_PALE, 0.0);
        d.rect(p, (PANEL, 0.0, PANEL, 816.0), CORAL, 0.0);
        d.text(p, (panel(0), 70.0, PANEL_W), l.pick("Útiles escolares", "School supplies"), font("Chewy", 34.0, CORAL));
        d.text(p, (panel(0), 420.0, PANEL_W), l.pick("Todo lo que buscas en útiles escolares está en nuestra tienda: calidad y buenos precios.", "Everything you need for school is in our shop: quality and fair prices."), font("Montserrat", 15.0, INK).line(1.6));
        d.rect(p, (panel(0) + 50.0, 170.0, 190.0, 200.0), MINT, 24.0);
        d.rect(p, (panel(0) + 80.0, 140.0, 130.0, 60.0), MINT_DARK, 20.0);
        d.polygon(p, (PANEL + 100.0, 80.0, 150.0, 140.0), WHITE, &[(0.5, 1.0), (0.0, 0.35), (0.15, 0.0), (0.5, 0.2), (0.85, 0.0), (1.0, 0.35)], 1.0);
        d.text(p, (panel(1), 300.0, PANEL_W), l.pick("Contáctanos", "Contact us"), font("Chewy", 44.0, WHITE).center());
        d.text(p, (panel(1), 400.0, PANEL_W), "+34 600 000 000\nhola@papeleria.com\nCalle del Sol 8", font("Montserrat", 15.0, WHITE).center().line(1.8));
        d.burst(p, (2.0 * PANEL + 110.0, 90.0, 140.0, 140.0), CORAL, 12, 0.32);
        d.ellipse(p, (2.0 * PANEL + 140.0, 120.0, 80.0, 80.0), "#FFC9BF", 1.0);
        d.text(p, (panel(2), 320.0, PANEL_W), l.pick("La Frida", "La Frida"), font("Shrikhand", 58.0, CORAL).center());
        d.text(p, (panel(2), 420.0, PANEL_W), l.pick("Papelería creativa", "Creative stationery"), font("Chewy", 28.0, MINT_DARK).center());
        d.rect(p, (2.0 * PANEL + 120.0, 560.0, 120.0, 120.0), MINT, 24.0);
        Doc::turn_last(p, 15.0);
    });
    d.page(TRIFOLD, pattern("grid", MIST, WHITE), |d, p| {
        let items = [
            (
                CORAL,
                l.pick("Cuadernos", "Notebooks"),
                l.pick(
                    "Rayados, cuadriculados y lisos, de todos los tamaños.",
                    "Ruled, squared and plain, in every size.",
                ),
            ),
            (
                MINT,
                l.pick("Arte", "Art"),
                l.pick(
                    "Lápices de colores, acuarelas y papeles especiales.",
                    "Coloured pencils, watercolours and special papers.",
                ),
            ),
            (
                INK,
                l.pick("Mochilas", "Backpacks"),
                l.pick(
                    "Cómodas, resistentes y con diseños divertidos.",
                    "Comfy, sturdy and with fun designs.",
                ),
            ),
        ];
        for (i, (color, title, body)) in items.iter().enumerate() {
            let x = panel(i as u32);
            d.rect(p, (x, 80.0, PANEL_W, 640.0), WHITE, 28.0);
            d.frame(p, (x, 80.0, PANEL_W, 640.0), color, 4.0, 28.0);
            d.ellipse(
                p,
                (x + PANEL_W / 2.0 - 60.0, 130.0, 120.0, 120.0),
                color,
                1.0,
            );
            d.text(
                p,
                (x + 20.0, 300.0, PANEL_W - 40.0),
                title,
                font("Chewy", 40.0, INK).center(),
            );
            d.text(
                p,
                (x + 24.0, 390.0, PANEL_W - 48.0),
                body,
                font("Montserrat", 15.0, SLATE).center().line(1.6),
            );
        }
    });
    d.document()
}

fn marketing_agency(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(TRIFOLD, solid(WHITE), |d, p| {
        d.text(
            p,
            (panel(0), 60.0, PANEL_W),
            l.pick(
                "GESTIÓN DE REDES SOCIALES Y CAMPAÑAS",
                "SOCIAL MEDIA AND CAMPAIGN MANAGEMENT",
            ),
            font("Montserrat", 17.0, INK).bold().line(1.3),
        );
        d.text(
            p,
            (panel(0), 170.0, PANEL_W),
            l.pick("RAZONES PARA TRABAJAR JUNTOS", "WHY WORK WITH US"),
            font("Montserrat", 15.0, INK).bold(),
        );
        let reasons = [
            l.pick("Estrategia a medida", "A tailored strategy"),
            l.pick("Contenido que conecta", "Content that connects"),
            l.pick("Resultados medibles", "Measurable results"),
            l.pick("Equipo cercano", "A friendly team"),
        ];
        for (i, r) in reasons.iter().enumerate() {
            let y = 230.0 + i as f64 * 120.0;
            d.ellipse(p, (panel(0), y, 44.0, 44.0), MINT, 1.0);
            d.text(
                p,
                (panel(0), y + 10.0, 44.0),
                &format!("0{}", i + 1),
                font("Montserrat", 15.0, INK).bold().center(),
            );
            d.text(
                p,
                (panel(0) + 60.0, y + 8.0, PANEL_W - 60.0),
                r,
                font("Montserrat", 15.0, SLATE),
            );
        }
        d.rect(p, (PANEL, 0.0, PANEL, 816.0), MINT_PALE, 0.0);
        d.text(
            p,
            (panel(1), 200.0, PANEL_W),
            l.pick("CONTÁCTANOS", "CONTACT US"),
            font("Montserrat", 22.0, INK).bold().center().spaced(3.0),
        );
        for (i, line) in ["+34 600 000 000", "@estudioshonos", "hola@shonos.com"]
            .iter()
            .enumerate()
        {
            let y = 270.0 + i as f64 * 64.0;
            d.rect(p, (panel(1) + 20.0, y, PANEL_W - 40.0, 44.0), MINT, 22.0);
            d.text(
                p,
                (panel(1) + 20.0, y + 11.0, PANEL_W - 40.0),
                line,
                font("Montserrat", 14.0, INK).bold().center(),
            );
        }
        d.burst(p, (panel(1) + 94.0, 560.0, 100.0, 100.0), MINT_DARK, 8, 0.3);
        d.text(
            p,
            (panel(1), 690.0, PANEL_W),
            l.pick("ESTUDIO SHONOS", "SHONOS STUDIO"),
            font("Montserrat", 22.0, INK).bold().center(),
        );
        d.rect(p, (2.0 * PANEL + 106.0, 90.0, 140.0, 140.0), MINT, 32.0);
        d.ellipse(p, (2.0 * PANEL + 146.0, 130.0, 60.0, 60.0), WHITE, 1.0);
        d.text(
            p,
            (panel(2), 300.0, PANEL_W),
            l.pick(
                "SERVICIOS DE\nMARKETING Y\nCOMUNICACIÓN",
                "MARKETING AND\nCOMMUNICATION\nSERVICES",
            ),
            font("Montserrat", 26.0, INK).bold().center().line(1.2),
        );
        d.text(
            p,
            (panel(2), 470.0, PANEL_W),
            l.pick(
                "Más de 100 marcas confían en nosotros.",
                "Over 100 brands trust us.",
            ),
            font("Montserrat", 14.0, SLATE).center(),
        );
    });
    d.page(TRIFOLD, solid(WHITE), |d, p| {
        let plans = [
            (
                l.pick("Básico", "Basic"),
                "€290",
                l.pick(
                    "3 redes · 12 publicaciones al mes · informe mensual",
                    "3 networks · 12 posts a month · monthly report",
                ),
            ),
            (
                l.pick("Crecimiento", "Growth"),
                "€590",
                l.pick(
                    "5 redes · 24 publicaciones · anuncios · informe quincenal",
                    "5 networks · 24 posts · ads · fortnightly report",
                ),
            ),
            (
                l.pick("Total", "Full"),
                "€990",
                l.pick(
                    "Todo lo anterior · vídeo · gestión de comunidad",
                    "All of the above · video · community management",
                ),
            ),
        ];
        for (i, (name, price, body)) in plans.iter().enumerate() {
            let x = panel(i as u32);
            let back = if i == 1 { INK } else { MINT_PALE };
            let fore = if i == 1 { WHITE } else { INK };
            d.rect(p, (x, 80.0, PANEL_W, 640.0), back, 24.0);
            d.text(
                p,
                (x + 20.0, 140.0, PANEL_W - 40.0),
                name,
                font("Montserrat", 30.0, fore).bold().center(),
            );
            d.text(
                p,
                (x + 20.0, 230.0, PANEL_W - 40.0),
                price,
                font("Archivo Black", 64.0, if i == 1 { MINT } else { MINT_DARK }).center(),
            );
            d.text(
                p,
                (x + 24.0, 380.0, PANEL_W - 48.0),
                body,
                font("Montserrat", 15.0, fore).center().line(1.7),
            );
        }
    });
    d.document()
}

fn travel_agency(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(TRIFOLD, solid(MINT_PALE), |d, p| {
        d.text(p, (panel(0), 60.0, PANEL_W), l.pick("HISTORIA", "HISTORY"), font("League Spartan", 34.0, INK).bold().spaced(3.0));
        d.text(p, (panel(0), 110.0, PANEL_W), l.pick("de la ciudad", "of the city"), font("Alice", 18.0, SLATE));
        d.text(p, (panel(0), 170.0, PANEL_W), l.pick("Calles con siglos de historia, mercados llenos de color y una cocina que se recuerda. Te llevamos a conocerla con guías locales.", "Centuries-old streets, colourful markets and food you won't forget. Local guides show you around."), font("Alice", 15.0, INK).line(1.7));
        photo(d, p, (panel(0), 520.0, PANEL_W, 230.0), MINT_TINT);
        d.rect(p, (PANEL, 0.0, PANEL, 816.0), CANVAS, 0.0);
        d.polygon(p, (PANEL + 116.0, 260.0, 120.0, 120.0), MINT_DARK, &[(0.0, 0.55), (1.0, 0.0), (0.6, 1.0), (0.45, 0.6)], 1.0);
        d.text(p, (panel(1), 420.0, PANEL_W), l.pick("BORCELLE", "BORCELLE"), font("League Spartan", 34.0, INK).bold().center().spaced(6.0));
        d.text(p, (panel(1), 470.0, PANEL_W), l.pick("Agencia de viajes", "Travel agency"), font("Alice", 16.0, SLATE).center());
        d.text(p, (panel(2), 60.0, PANEL_W), l.pick("VISITA LA CIUDAD DE", "VISIT THE CITY OF"), font("League Spartan", 16.0, INK).bold().right().spaced(3.0));
        d.text(p, (panel(2), 90.0, PANEL_W), l.pick("MÉXICO", "MEXICO"), font("League Spartan", 60.0, INK).bold().right());
        photo(d, p, (2.0 * PANEL, 200.0, PANEL, 420.0), MINT_TINT);
        d.text(p, (panel(2), 680.0, PANEL_W), l.pick("Paquetes desde 899 €", "Packages from €899"), font("League Spartan", 22.0, INK).bold().right());
    });
    d.page(TRIFOLD, solid(WHITE), |d, p| {
        let days = [
            (
                l.pick("Día 1", "Day 1"),
                l.pick("Centro histórico", "Old town"),
                l.pick(
                    "Paseo por la plaza mayor, la catedral y los museos del centro.",
                    "A walk around the main square, the cathedral and the museums.",
                ),
            ),
            (
                l.pick("Día 2", "Day 2"),
                l.pick("Mercados y sabores", "Markets and flavours"),
                l.pick(
                    "Ruta por mercados con degustación de cocina tradicional.",
                    "A market route with a taste of traditional food.",
                ),
            ),
            (
                l.pick("Día 3", "Day 3"),
                l.pick("Canales y jardines", "Canals and gardens"),
                l.pick(
                    "Paseo en barca y tarde libre en los jardines flotantes.",
                    "A boat ride and a free afternoon in the floating gardens.",
                ),
            ),
        ];
        for (i, (day, title, body)) in days.iter().enumerate() {
            let x = panel(i as u32);
            photo(
                d,
                p,
                (x, 60.0, PANEL_W, 300.0),
                if i == 1 { CORAL_LIGHT } else { MINT_TINT },
            );
            d.text(
                p,
                (x, 400.0, PANEL_W),
                day,
                font("League Spartan", 18.0, MINT_DARK).bold().spaced(3.0),
            );
            d.text(
                p,
                (x, 440.0, PANEL_W),
                title,
                font("League Spartan", 30.0, INK).bold(),
            );
            d.text(
                p,
                (x, 510.0, PANEL_W),
                body,
                font("Alice", 16.0, SLATE).line(1.7),
            );
        }
    });
    d.document()
}

// ------------------------------------------------ Instagram posts (4:5)

const POST: (f64, f64, &str) = (1080.0, 1350.0, "px");

/// A photo placeholder: a soft tile with a sun and two hills.
fn photo(d: &mut Doc, p: &mut Page, (x, y, w, h): (f64, f64, f64, f64), back: &str) {
    d.rect(p, (x, y, w, h), back, 0.0);
    d.ellipse(
        p,
        (x + w * 0.65, y + h * 0.15, w * 0.18, w * 0.18),
        WHITE,
        0.9,
    );
    d.polygon(
        p,
        (x, y + h * 0.45, w, h * 0.55),
        MINT,
        &[
            (0.0, 1.0),
            (0.0, 0.5),
            (0.35, 0.1),
            (0.7, 0.6),
            (1.0, 0.3),
            (1.0, 1.0),
        ],
        0.9,
    );
    d.photo_frame(p, (x, y, w, h), "rect", 0.0);
}

/// A round photo: a soft disc and an empty round frame over it.
fn circle_photo(d: &mut Doc, p: &mut Page, (x, y, w, h): (f64, f64, f64, f64), back: &str) {
    d.ellipse(p, (x, y, w, h), back, 1.0);
    d.ellipse(p, (x + w * 0.3, y + h * 0.22, w * 0.4, h * 0.4), WHITE, 0.8);
    d.photo_frame(p, (x, y, w, h), "ellipse", 0.0);
}

/// A photo with rounded corners of `radius`.
fn rounded_photo(
    d: &mut Doc,
    p: &mut Page,
    (x, y, w, h): (f64, f64, f64, f64),
    back: &str,
    radius: f64,
) {
    d.rect(p, (x, y, w, h), back, radius);
    d.photo_frame(p, (x, y, w, h), "rect", radius);
}

/// A photo turned by `degrees`, as a print laid on the page.
fn turned_photo(
    d: &mut Doc,
    p: &mut Page,
    (x, y, w, h): (f64, f64, f64, f64),
    back: &str,
    degrees: f64,
) {
    d.rect(p, (x, y, w, h), back, 0.0);
    Doc::turn_last(p, degrees);
    d.photo_frame(p, (x, y, w, h), "rect", 0.0);
    Doc::turn_last(p, degrees);
}

fn new_hobbies(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(INK), |d, p| {
        d.text(
            p,
            (60.0, 60.0, 960.0),
            l.pick(
                "Mis nuevos hobbies\npara este mes",
                "My new hobbies\nfor this month",
            ),
            font("Pacifico", 74.0, CORAL_LIGHT).center().line(0.9),
        );
        d.rect(p, (110.0, 330.0, 860.0, 880.0), CORAL, 0.0);
        photo(d, p, (150.0, 370.0, 780.0, 800.0), MINT_TINT);
        d.star(p, (820.0, 280.0, 160.0, 160.0), MINT);
        d.star(p, (70.0, 1080.0, 180.0, 180.0), MINT);
        d.star(p, (940.0, 700.0, 70.0, 70.0), WHITE);
        d.rect(p, (600.0, 1110.0, 400.0, 70.0), CORAL_LIGHT, 35.0);
        d.text(
            p,
            (600.0, 1128.0, 400.0),
            l.pick("Cosas bonitas que hacer", "Lovely things to do"),
            font("Inter", 26.0, INK).center(),
        );
        d.text(
            p,
            (600.0, 1240.0, 400.0),
            "@tumarca",
            font("Inter", 24.0, MIST).right(),
        );
    });
    d.document()
}

fn creative_routine(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(INK_LIGHT), |d, p| {
        d.text(
            p,
            (60.0, 70.0, 960.0),
            l.pick("RESUMEN DEL DÍA", "DAILY RECAP"),
            font("Montserrat", 24.0, MINT_TINT)
                .bold()
                .center()
                .spaced(6.0),
        );
        d.text(
            p,
            (60.0, 120.0, 960.0),
            l.pick(
                "mi rutina como\ndirectora creativa",
                "my routine as a\ncreative director",
            ),
            font("Poppins", 70.0, MINT).bold().center().line(0.95),
        );
        d.text(
            p,
            (60.0, 330.0, 960.0),
            l.pick("[ realista ]", "[ for real ]"),
            font("Poppins", 36.0, CORAL_LIGHT).center(),
        );
        d.rect(p, (240.0, 520.0, 600.0, 380.0), INK, 18.0);
        d.rect(p, (270.0, 550.0, 540.0, 320.0), MINT_TINT, 6.0);
        d.rect(p, (180.0, 900.0, 720.0, 30.0), MIST, 15.0);
        d.star(p, (110.0, 470.0, 70.0, 70.0), MINT);
        d.star(p, (900.0, 800.0, 70.0, 70.0), MINT);
        d.rect(p, (0.0, 1150.0, 1080.0, 200.0), MINT, 0.0);
        d.text(
            p,
            (60.0, 1230.0, 480.0),
            l.pick("agencia\ncreativa", "creative\nagency"),
            font("Montserrat", 24.0, INK).bold().line(1.0),
        );
        d.text(
            p,
            (540.0, 1230.0, 480.0),
            "@tumarca\nwww.tumarca.com",
            font("Montserrat", 24.0, INK).bold().right().line(1.0),
        );
    });
    d.document()
}

fn new_collection(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(MINT_DARK), |d, p| {
        photo(d, p, (0.0, 0.0, 1080.0, 1350.0), MINT_PALE);
        d.rect(p, (0.0, 0.0, 1080.0, 420.0), MINT_DARK, 0.0);
        d.rect(p, (0.0, 1080.0, 1080.0, 270.0), MINT_DARK, 0.0);
        d.text(
            p,
            (60.0, 90.0, 960.0),
            l.pick("NUEVA\nCOLECCIÓN", "NEW\nCOLLECTION"),
            font("DM Serif Display", 120.0, WHITE)
                .italic()
                .center()
                .line(0.85),
        );
        d.text(
            p,
            (60.0, 1120.0, 960.0),
            l.pick("VISITA NUESTRO CATÁLOGO", "SEE OUR CATALOGUE"),
            font("Cormorant Garamond", 34.0, WHITE)
                .bold()
                .center()
                .spaced(6.0),
        );
        d.rect(p, (340.0, 1210.0, 400.0, 64.0), WHITE, 32.0);
        d.text(
            p,
            (340.0, 1225.0, 400.0),
            "www.tumarca.com",
            font("Inter", 24.0, INK).bold().center(),
        );
    });
    d.document()
}

fn month_in_numbers(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(CANVAS), |d, p| {
        d.star(p, (40.0, 40.0, 260.0, 260.0), CORAL_LIGHT);
        d.star(p, (800.0, 1080.0, 240.0, 240.0), MINT_TINT);
        d.text(
            p,
            (60.0, 260.0, 960.0),
            l.pick("MI MES EN", "MY MONTH IN"),
            font("Archivo Black", 80.0, CORAL).center(),
        );
        d.text(
            p,
            (60.0, 380.0, 960.0),
            l.pick("NÚMEROS", "NUMBERS"),
            font("Archivo Black", 110.0, INK).center().spaced(16.0),
        );
        let stats = [
            ("12", l.pick("libros leídos", "books read")),
            ("48", l.pick("km corridos", "km run")),
            ("3", l.pick("proyectos nuevos", "new projects")),
        ];
        for (i, (n, label)) in stats.iter().enumerate() {
            let y = 620.0 + i as f64 * 200.0;
            d.rect(
                p,
                (120.0, y, 840.0, 160.0),
                if i == 1 { MINT_TINT } else { WHITE },
                28.0,
            );
            d.text(
                p,
                (160.0, y + 20.0, 260.0),
                n,
                font("Archivo Black", 90.0, CORAL),
            );
            d.text(
                p,
                (440.0, y + 50.0, 480.0),
                label,
                font("Poppins", 42.0, INK),
            );
        }
    });
    d.document()
}

fn happy_birthday(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(CORAL_LIGHT), |d, p| {
        d.ring(p, (-300.0, 600.0, 900.0, 900.0), "#FFC9BF", 70.0);
        d.ring(p, (600.0, 900.0, 700.0, 700.0), "#FFC9BF", 60.0);
        d.text(
            p,
            (60.0, 110.0, 960.0),
            l.pick("Feliz", "Happy"),
            font("Great Vibes", 160.0, INK).center(),
        );
        d.text(
            p,
            (60.0, 330.0, 960.0),
            l.pick("CUMPLEAÑOS", "BIRTHDAY"),
            font("Cormorant Garamond", 120.0, INK).center().spaced(6.0),
        );
        d.rect(p, (340.0, 830.0, 400.0, 180.0), WHITE, 30.0);
        d.rect(p, (390.0, 700.0, 300.0, 150.0), CORAL, 30.0);
        for i in 0..3 {
            d.rect(
                p,
                (440.0 + f64::from(i) * 90.0, 620.0, 20.0, 90.0),
                MINT,
                10.0,
            );
            d.ellipse(
                p,
                (436.0 + f64::from(i) * 90.0, 585.0, 28.0, 40.0),
                CANVAS,
                1.0,
            );
        }
        d.text(
            p,
            (60.0, 1180.0, 960.0),
            "@tumarca",
            font("Inter", 28.0, INK).center(),
        );
    });
    d.document()
}

fn launch_promo(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(CANVAS), |d, p| {
        d.text(
            p,
            (60.0, 60.0, 960.0),
            l.pick("PROMO LANZAMIENTO", "LAUNCH OFFER"),
            font("Montserrat", 30.0, SLATE).center().spaced(8.0),
        );
        d.rect(p, (0.0, 150.0, 1080.0, 1050.0), MIST, 0.0);
        for i in 0..4 {
            let y = 240.0 + f64::from(i) * 230.0;
            d.ring(
                p,
                (120.0 + f64::from(i % 2) * 120.0, y, 180.0, 180.0),
                MINT_DARK,
                26.0,
            );
        }
        d.rect(p, (560.0, 450.0, 520.0, 520.0), CORAL, 0.0);
        d.text(
            p,
            (600.0, 560.0, 440.0),
            l.pick("Set\ndorado", "Golden\nset"),
            font("Cormorant Garamond", 110.0, WHITE).center().line(0.95),
        );
        d.text(
            p,
            (600.0, 860.0, 440.0),
            "-30%",
            font("Montserrat", 40.0, WHITE).bold().center(),
        );
        d.text(
            p,
            (60.0, 1250.0, 960.0),
            "@tumarca",
            font("Cormorant Garamond", 30.0, SLATE).center(),
        );
    });
    d.document()
}

fn sticky_note(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, pattern("grid_fine", MIST, WHITE), |d, p| {
        for i in 0..10 {
            d.ring(
                p,
                (30.0, 120.0 + f64::from(i) * 120.0, 60.0, 60.0),
                SLATE,
                6.0,
            );
        }
        d.text(
            p,
            (180.0, 120.0, 820.0),
            "LAURA DEL VALLE",
            font("Courgette", 30.0, INK).center(),
        );
        d.rect(p, (480.0, 165.0, 240.0, 2.0), INK, 0.0);
        d.text(
            p,
            (220.0, 210.0, 760.0),
            l.pick(
                "Últimos días para sumarte al reto creativo de este verano.",
                "Last days to join this summer's creative challenge.",
            ),
            font("Courgette", 26.0, SLATE).center().line(1.4),
        );
        d.rect(p, (330.0, 520.0, 620.0, 620.0), MINT_TINT, 0.0);
        Doc::turn_last(p, -6.0);
        d.rect(p, (590.0, 470.0, 50.0, 140.0), BORDER, 25.0);
        Doc::turn_last(p, -6.0);
        d.text(
            p,
            (390.0, 660.0, 500.0),
            l.pick(
                "¿Cuándo fue la\núltima vez que\nte tomaste el\ndía libre?",
                "When did you\nlast take\na day\noff?",
            ),
            font("Kaushan Script", 52.0, INK).center().line(1.1),
        );
        Doc::turn_last(p, -6.0);
    });
    d.document()
}

fn small_habits(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(
        POST,
        json!({ "gradient": linear(90.0, MINT, CANVAS) }),
        |d, p| {
            d.ellipse(p, (-120.0, 900.0, 500.0, 300.0), WHITE, 0.8);
            d.ellipse(p, (700.0, 160.0, 520.0, 280.0), WHITE, 0.8);
            d.rect(p, (90.0, 430.0, 900.0, 470.0), CANVAS, 6.0);
            Doc::turn_last(p, -3.0);
            d.text(
                p,
                (130.0, 470.0, 820.0),
                l.pick("Pequeños", "Small"),
                font("Bebas Neue", 70.0, INK).center(),
            );
            d.text(
                p,
                (130.0, 540.0, 820.0),
                l.pick("hábitos", "habits"),
                font("DM Serif Display", 150.0, INK).center(),
            );
            d.text(
                p,
                (130.0, 730.0, 820.0),
                l.pick("que hacen la diferencia", "that make a difference"),
                font("Cormorant Garamond", 52.0, INK).italic().center(),
            );
            d.text(
                p,
                (130.0, 820.0, 820.0),
                l.pick("← desliza y descubre", "← swipe to see"),
                font("Cormorant Garamond", 34.0, SLATE).center(),
            );
        },
    );
    d.document()
}

fn which_team(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(CORAL), |d, p| {
        d.ring(p, (440.0, -80.0, 200.0, 200.0), CANVAS, 6.0);
        d.burst(p, (480.0, 140.0, 120.0, 120.0), CANVAS, 8, 0.3);
        d.text(
            p,
            (60.0, 420.0, 960.0),
            l.pick("¿QUÉ\nTEAM\nERES?", "WHICH\nTEAM\nARE YOU?"),
            font("Cormorant Garamond", 160.0, WHITE).center().line(0.8),
        );
        d.text(
            p,
            (60.0, 980.0, 960.0),
            l.pick("VERSIÓN: ACCESORIOS", "EDITION: ACCESSORIES"),
            font("Montserrat", 30.0, CANVAS).center().spaced(4.0),
        );
        d.text(
            p,
            (60.0, 1220.0, 960.0),
            "@tumarca",
            font("Inter", 28.0, CANVAS).center(),
        );
    });
    d.document()
}

fn skincare(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(POST, solid(MIST), |d, p| {
        d.rect(p, (180.0, 120.0, 720.0, 300.0), CANVAS, 8.0);
        d.text(
            p,
            (180.0, 150.0, 720.0),
            "Skincare",
            font("Belleza", 100.0, INK).center(),
        );
        d.text(
            p,
            (180.0, 290.0, 720.0),
            l.pick("RUTINA", "ROUTINE"),
            font("Montserrat", 40.0, INK).center().spaced(14.0),
        );
        d.text(
            p,
            (180.0, 360.0, 720.0),
            l.pick(
                "Aceites esenciales para tu piel",
                "Essential oils for your skin",
            ),
            font("Inter", 24.0, SLATE).center(),
        );
        d.rect(p, (300.0, 780.0, 200.0, 380.0), MINT_DARK, 30.0);
        d.rect(p, (350.0, 700.0, 100.0, 100.0), INK, 12.0);
        d.rect(p, (330.0, 900.0, 140.0, 160.0), CANVAS, 8.0);
        d.rect(p, (580.0, 840.0, 200.0, 320.0), MINT, 30.0);
        d.rect(p, (630.0, 760.0, 100.0, 100.0), INK, 12.0);
        d.rect(p, (610.0, 940.0, 140.0, 140.0), CANVAS, 8.0);
        d.rect(p, (0.0, 1160.0, 1080.0, 190.0), MINT_TINT, 0.0);
    });
    d.document()
}

// ------------------------------------------------ invitations (105 × 148 mm)

const CARD: (f64, f64, &str) = (105.0, 148.0, "mm");
/// The card in px.
const CW: f64 = 105.0 * MM;
const CH: f64 = 148.0 * MM;

/// The date, time and place of a party, centred in `w` from `(x, y)`.
fn party_details(
    d: &mut Doc,
    p: &mut Page,
    (x, y, w): (f64, f64, f64),
    l: Lang,
    color: &'static str,
) {
    d.text(
        p,
        (x, y, w),
        l.pick("20  |  SEP  |  19:00 H", "20  |  SEP  |  7 PM"),
        font("Montserrat", 13.0, color).bold().center().spaced(2.0),
    );
    d.text(
        p,
        (x, y + 26.0, w),
        l.pick("Calle Mayor 123, Madrid", "123 Main Street, London"),
        font("Montserrat", 8.5, color).center().spaced(1.0),
    );
}

fn oval_party(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, pattern("dots", CORAL, MIST), |d, p| {
        d.ellipse(p, (34.0, 50.0, CW - 68.0, CH - 100.0), CANVAS, 1.0);
        d.ring(p, (34.0, 50.0, CW - 68.0, CH - 100.0), INK, 5.0);
        d.ring(p, (46.0, 62.0, CW - 92.0, CH - 124.0), CORAL, 1.5);
        d.text(
            p,
            (60.0, 120.0, CW - 120.0),
            l.pick("ESTÁS INVITADA A MI", "YOU'RE INVITED TO MY"),
            font("Montserrat", 8.0, INK).bold().center().spaced(2.0),
        );
        d.text(
            p,
            (60.0, 140.0, CW - 120.0),
            l.pick("Fiesta de\ncumpleaños", "Birthday\nparty"),
            font("Great Vibes", 40.0, INK).center().line(0.85),
        );
        d.text(
            p,
            (60.0, 226.0, CW - 120.0),
            l.pick("Los 23 de Isabel", "Isabel turns 23"),
            font("Dancing Script", 20.0, CORAL).center(),
        );
        party_details(d, p, (60.0, 270.0, CW - 120.0), l, INK);
        circle_photo(d, p, (CW / 2.0 - 42.0, 330.0, 84.0, 84.0), MINT_TINT);
        // The frame draws its own outline: nothing lies over it.
        Doc::outline_last(p, INK, 2.0);
        d.text(
            p,
            (60.0, 430.0, CW - 120.0),
            l.pick(
                "Confirma tu asistencia al\n600 000 000",
                "Please reply to\n600 000 000",
            ),
            font("Montserrat", 8.0, INK).center().line(1.4),
        );
    });
    d.document()
}

fn striped_party(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, pattern("stripes", CORAL, "#FFC9BF"), |d, p| {
        d.rect(p, (36.0, 46.0, CW - 72.0, CH - 92.0), "#FFE3DE", 60.0);
        d.frame(p, (46.0, 56.0, CW - 92.0, CH - 112.0), CORAL, 2.0, 52.0);
        d.text(
            p,
            (60.0, 110.0, CW - 120.0),
            l.pick("Mi cumple", "My birthday"),
            font("Pacifico", 34.0, CORAL).center(),
        );
        d.text(
            p,
            (60.0, 170.0, CW - 120.0),
            "25",
            font("Shrikhand", 96.0, CORAL).center(),
        );
        d.text(
            p,
            (60.0, 320.0, CW - 120.0),
            l.pick("12 | SEP | 21 HS", "12 | SEP | 9 PM"),
            font("Chewy", 24.0, CORAL).center(),
        );
        d.text(
            p,
            (60.0, 360.0, CW - 120.0),
            l.pick(
                "CALLE MAYOR 123,\nCUALQUIER LUGAR",
                "123 MAIN STREET,\nANYWHERE",
            ),
            font("Chewy", 14.0, CORAL).center().line(1.2),
        );
        d.text(
            p,
            (60.0, 440.0, CW - 120.0),
            "Luli",
            font("Dancing Script", 26.0, CORAL).center(),
        );
    });
    d.document()
}

fn twenty_first(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, solid(CORAL), |d, p| {
        d.rect(p, (26.0, 26.0, CW - 52.0, CH - 52.0), CANVAS, 4.0);
        d.frame(p, (36.0, 36.0, CW - 72.0, CH - 72.0), CORAL, 1.5, 2.0);
        d.text(
            p,
            (50.0, 90.0, CW - 100.0),
            l.pick("Cumplo\n21 años", "I'm turning\n21"),
            font("Dancing Script", 50.0, CORAL).center().line(0.9),
        );
        d.text(
            p,
            (50.0, 250.0, CW - 100.0),
            l.pick("17 OCTUBRE\n22 H", "OCTOBER 17\n10 PM"),
            font("Montserrat", 20.0, INK).bold().center().line(1.1),
        );
        d.text(
            p,
            (50.0, 330.0, CW - 100.0),
            l.pick(
                "CALLE MAYOR 123,\nCUALQUIER LUGAR",
                "123 MAIN STREET,\nANYWHERE",
            ),
            font("Montserrat", 10.0, INK).center().line(1.3),
        );
        d.text(
            p,
            (50.0, 390.0, CW - 100.0),
            l.pick(
                "confirmar asistencia\n600 000 000",
                "please reply\n600 000 000",
            ),
            font("Montserrat", 9.0, CORAL).center().line(1.3),
        );
        d.text(
            p,
            (50.0, 450.0, CW - 100.0),
            "Elena",
            font("Great Vibes", 30.0, INK).center(),
        );
        d.star(p, (50.0, 470.0, 26.0, 26.0), CORAL);
        d.star(p, (CW - 80.0, 60.0, 26.0, 26.0), CORAL);
    });
    d.document()
}

fn girls_night(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, solid(CANVAS), |d, p| {
        d.frame(p, (22.0, 22.0, CW - 44.0, CH - 44.0), CORAL, 2.0, 30.0);
        d.burst(p, (CW - 54.0, 12.0, 34.0, 34.0), CORAL, 4, 0.12);
        d.burst(p, (14.0, CH - 52.0, 34.0, 34.0), CORAL, 4, 0.12);
        d.text(
            p,
            (50.0, 70.0, CW - 100.0),
            l.pick("invitación", "invitation"),
            font("Dancing Script", 22.0, CORAL).center(),
        );
        d.text(
            p,
            (50.0, 104.0, CW - 100.0),
            l.pick(
                "Desconecta y recarga energías con nuestra",
                "Switch off and recharge with our",
            ),
            font("Montserrat", 8.0, SLATE).center(),
        );
        d.text(
            p,
            (40.0, 130.0, CW - 80.0),
            l.pick("Noche de\nAmigas", "Girls'\nNight"),
            font("DM Serif Display", 50.0, CORAL).center().line(0.9),
        );
        d.polygon(
            p,
            (CW / 2.0 - 40.0, 260.0, 80.0, 80.0),
            CORAL,
            &[
                (0.2, 0.0),
                (0.45, 0.0),
                (0.38, 0.5),
                (0.36, 1.0),
                (0.3, 1.0),
                (0.28, 0.5),
            ],
            1.0,
        );
        d.polygon(
            p,
            (CW / 2.0 - 30.0, 260.0, 80.0, 80.0),
            MINT_DARK,
            &[
                (0.55, 0.0),
                (0.8, 0.0),
                (0.72, 0.5),
                (0.7, 1.0),
                (0.64, 1.0),
                (0.62, 0.5),
            ],
            1.0,
        );
        d.text(
            p,
            (50.0, 352.0, CW - 100.0),
            l.pick(
                "No hay mejor terapia que una buena charla y la mejor compañía.",
                "No therapy beats a good chat and the best company.",
            ),
            font("Montserrat", 8.5, SLATE).center().line(1.4),
        );
        d.ring(p, (70.0, 400.0, CW - 140.0, 40.0), CORAL, 1.5);
        d.text(
            p,
            (70.0, 412.0, CW - 140.0),
            l.pick("Viernes  ·  12  ·  19:00 h", "Friday  ·  12  ·  7 PM"),
            font("Montserrat", 10.0, INK).bold().center(),
        );
        d.text(
            p,
            (50.0, 460.0, CW - 100.0),
            l.pick(
                "Calle Mayor 123 · Confirma al 600 000 000",
                "123 Main St · Reply to 600 000 000",
            ),
            font("Montserrat", 8.0, SLATE).center(),
        );
    });
    d.document()
}

fn julieta_25(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, pattern("stripes", MINT_TINT, WHITE), |d, p| {
        d.rect(p, (40.0, 40.0, CW - 80.0, CH - 80.0), WHITE, 150.0);
        d.frame(p, (50.0, 50.0, CW - 100.0, CH - 100.0), MINT, 2.0, 140.0);
        d.text(
            p,
            (60.0, 110.0, CW - 120.0),
            "25",
            font("Belleza", 56.0, INK).center(),
        );
        d.text(
            p,
            (60.0, 170.0, CW - 120.0),
            "Julieta",
            font("Cormorant Garamond", 58.0, INK).center(),
        );
        d.text(
            p,
            (70.0, 250.0, CW - 140.0),
            l.pick(
                "Este día tan especial estaremos junto a nuestros seres queridos.",
                "We'll spend this special day with the people we love.",
            ),
            font("Cormorant Garamond", 11.0, SLATE)
                .italic()
                .center()
                .line(1.3),
        );
        d.text(
            p,
            (70.0, 300.0, CW - 140.0),
            l.pick("SÁBADO 25  ·  22:00", "SATURDAY 25  ·  10 PM"),
            font("Montserrat", 10.0, INK).bold().center(),
        );
        d.burst(p, (CW / 2.0 - 16.0, 322.0, 32.0, 32.0), CORAL, 8, 0.3);
        for (i, line) in [l.pick("Calle Mayor 123", "123 Main Street"), "600 000 000"]
            .iter()
            .enumerate()
        {
            let y = 372.0 + i as f64 * 38.0;
            d.rect(p, (80.0, y, CW - 160.0, 28.0), MINT_TINT, 14.0);
            d.text(
                p,
                (80.0, y + 6.0, CW - 160.0),
                line,
                font("Cormorant Garamond", 12.0, INK).center(),
            );
        }
        d.text(
            p,
            (60.0, 455.0, CW - 120.0),
            l.pick("Te esperamos", "See you there"),
            font("Cormorant Garamond", 18.0, INK).italic().center(),
        );
    });
    d.document()
}

fn star_party(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, solid(CORAL_LIGHT), |d, p| {
        d.ellipse(p, (-60.0, -40.0, 220.0, 180.0), CORAL, 1.0);
        d.ellipse(p, (CW - 140.0, CH - 160.0, 240.0, 220.0), CORAL, 1.0);
        d.rect(p, (36.0, 60.0, CW - 72.0, CH - 120.0), CANVAS, 24.0);
        d.star(p, (CW - 90.0, 20.0, 80.0, 80.0), INK);
        d.star(p, (20.0, CH - 110.0, 70.0, 70.0), INK);
        d.text(
            p,
            (60.0, 110.0, CW - 120.0),
            "Bianca",
            font("Kaushan Script", 40.0, INK).center(),
        );
        d.text(
            p,
            (60.0, 160.0, CW - 120.0),
            l.pick("Cumpleaños", "Birthday"),
            font("Kaushan Script", 36.0, INK).center(),
        );
        d.rect(p, (CW / 2.0 - 40.0, 250.0, 80.0, 40.0), CORAL, 6.0);
        d.rect(p, (CW / 2.0 - 54.0, 286.0, 108.0, 44.0), INK, 6.0);
        for i in 0..3 {
            d.rect(
                p,
                (CW / 2.0 - 24.0 + f64::from(i) * 22.0, 228.0, 5.0, 24.0),
                MINT,
                2.0,
            );
        }
        d.text(
            p,
            (60.0, 350.0, CW - 120.0),
            l.pick("OCTUBRE", "OCTOBER"),
            font("Montserrat", 11.0, INK).bold().center().spaced(3.0),
        );
        d.text(
            p,
            (60.0, 370.0, CW - 120.0),
            l.pick("VIERNES   15   22:00 H", "FRIDAY   15   10 PM"),
            font("Montserrat", 16.0, INK).bold().center(),
        );
        d.text(
            p,
            (60.0, 400.0, CW - 120.0),
            l.pick("Calle Mayor 123", "123 Main Street"),
            font("Montserrat", 9.0, INK).center(),
        );
    });
    d.document()
}

fn big_birthday(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, solid(CANVAS), |d, p| {
        d.text(
            p,
            (30.0, 40.0, CW - 60.0),
            l.pick("TE INVITAMOS A FESTEJAR MI", "JOIN US TO CELEBRATE MY"),
            font("Montserrat", 10.0, INK).bold().spaced(2.0),
        );
        d.text(
            p,
            (30.0, 62.0, CW - 60.0),
            l.pick("CUMPLE 25", "25TH"),
            font("Bebas Neue", 96.0, CORAL),
        );
        d.text(
            p,
            (30.0, 200.0, 160.0),
            l.pick("VIERNES\n20 DE NOV\n20 H", "FRIDAY\nNOV 20\n8 PM"),
            font("Montserrat", 15.0, INK).bold().line(1.2),
        );
        d.rect(p, (180.0, 220.0, 170.0, 200.0), WHITE, 0.0);
        Doc::turn_last(p, 6.0);
        turned_photo(d, p, (192.0, 232.0, 146.0, 150.0), MINT_TINT, 6.0);
        Doc::turn_last(p, 6.0);
        d.polygon(
            p,
            (250.0, 190.0, 50.0, 70.0),
            CORAL,
            &[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)],
            1.0,
        );
        d.text(
            p,
            (30.0, 460.0, CW - 60.0),
            l.pick(
                "CALLE MAYOR 123, CUALQUIER LUGAR",
                "123 MAIN STREET, ANYWHERE",
            ),
            font("Montserrat", 9.0, INK).spaced(1.0),
        );
        d.star(p, (40.0, 320.0, 40.0, 40.0), CORAL);
    });
    d.document()
}

fn thirty(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, solid(MINT_DARK), |d, p| {
        d.ring(p, (CW / 2.0 - 130.0, 110.0, 260.0, 260.0), MINT, 2.0);
        d.text(
            p,
            (30.0, 80.0, CW - 60.0),
            l.pick("Celebremos juntos mi", "Let's celebrate my"),
            font("Cormorant Garamond", 18.0, CANVAS).italic().center(),
        );
        d.text(
            p,
            (30.0, 120.0, CW - 60.0),
            "30",
            font("DM Serif Display", 180.0, CANVAS).center(),
        );
        d.text(
            p,
            (30.0, 375.0, CW - 60.0),
            l.pick("Cumpleaños", "Birthday"),
            font("Great Vibes", 50.0, CORAL_LIGHT).center(),
        );
        d.text(
            p,
            (30.0, 460.0, CW - 60.0),
            l.pick(
                "SÁBADO 14 · 21 H\nCALLE MAYOR 123",
                "SATURDAY 14 · 9 PM\n123 MAIN STREET",
            ),
            font("Montserrat", 10.0, CANVAS)
                .center()
                .spaced(2.0)
                .line(1.4),
        );
    });
    d.document()
}

fn wedding(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, solid(CANVAS), |d, p| {
        for (x, y, turn) in [
            (20.0, 20.0, -30.0),
            (60.0, 0.0, 10.0),
            (CW - 110.0, CH - 100.0, 150.0),
            (CW - 70.0, CH - 70.0, 200.0),
        ] {
            d.polygon(
                p,
                (x, y, 60.0, 100.0),
                MINT,
                &[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)],
                0.8,
            );
            Doc::turn_last(p, turn);
        }
        d.text(
            p,
            (40.0, 120.0, CW - 80.0),
            l.pick("NOS CASAMOS", "WE'RE GETTING MARRIED"),
            font("Montserrat", 10.0, MINT_DARK)
                .bold()
                .center()
                .spaced(4.0),
        );
        d.text(
            p,
            (40.0, 160.0, CW - 80.0),
            "Ana\n&\nLuis",
            font("Great Vibes", 54.0, INK).center().line(0.8),
        );
        d.rect(p, (CW / 2.0 - 40.0, 340.0, 80.0, 1.5), MINT_DARK, 0.0);
        d.text(
            p,
            (40.0, 360.0, CW - 80.0),
            l.pick("SÁBADO 12 DE JUNIO\n18:00 H", "SATURDAY, JUNE 12\n6 PM"),
            font("Cormorant Garamond", 16.0, INK)
                .bold()
                .center()
                .line(1.2),
        );
        d.text(
            p,
            (40.0, 420.0, CW - 80.0),
            l.pick("Finca El Olivar, Toledo", "The Olive Grove, Kent"),
            font("Cormorant Garamond", 13.0, SLATE).italic().center(),
        );
    });
    d.document()
}

fn baby_shower(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(CARD, solid(MINT_PALE), |d, p| {
        for (x, y, w) in [
            (-30.0, 60.0, 160.0),
            (CW - 130.0, 30.0, 170.0),
            (40.0, CH - 120.0, 200.0),
            (CW - 150.0, CH - 80.0, 190.0),
        ] {
            d.ellipse(p, (x, y, w, w * 0.45), WHITE, 1.0);
        }
        d.ellipse(p, (CW / 2.0 - 50.0, 130.0, 100.0, 100.0), "#FFC9BF", 1.0);
        d.text(
            p,
            (40.0, 255.0, CW - 80.0),
            "Baby\nshower",
            font("Pacifico", 44.0, MINT_DARK).center().line(0.9),
        );
        d.text(
            p,
            (40.0, 380.0, CW - 80.0),
            l.pick("en honor de Martina", "for baby Martina"),
            font("Dancing Script", 20.0, INK).center(),
        );
        d.text(
            p,
            (40.0, 420.0, CW - 80.0),
            l.pick(
                "DOMINGO 8 · 12:00 H\nCALLE MAYOR 123",
                "SUNDAY 8 · NOON\n123 MAIN STREET",
            ),
            font("Montserrat", 10.0, INK)
                .bold()
                .center()
                .line(1.4)
                .spaced(1.0),
        );
    });
    d.document()
}

// ------------------------------------------- Instagram posts (square)

const SQUARE: (f64, f64, &str) = (1080.0, 1080.0, "px");

fn tip_of_the_day(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(CANVAS), |d, p| {
        d.rect(p, (0.0, 0.0, 1080.0, 260.0), INK, 0.0);
        d.text(
            p,
            (80.0, 80.0, 920.0),
            l.pick("CONSEJO DEL DÍA", "TIP OF THE DAY"),
            font("Montserrat", 52.0, WHITE).bold().spaced(8.0),
        );
        d.text(
            p,
            (80.0, 330.0, 300.0),
            "#07",
            font("Archivo Black", 150.0, CORAL),
        );
        d.text(
            p,
            (80.0, 560.0, 920.0),
            l.pick(
                "Deja respirar tu diseño:\nel espacio vacío también comunica.",
                "Let your design breathe:\nempty space speaks too.",
            ),
            font("Poppins", 52.0, INK).bold().line(1.1),
        );
        d.rect(p, (80.0, 900.0, 160.0, 10.0), CORAL, 5.0);
        d.text(
            p,
            (600.0, 960.0, 400.0),
            "@tumarca",
            font("Inter", 28.0, SLATE).right(),
        );
    });
    d.document()
}

fn before_after(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(MIST), |d, p| {
        d.text(
            p,
            (60.0, 60.0, 960.0),
            l.pick("ANTES Y DESPUÉS", "BEFORE AND AFTER"),
            font("Bebas Neue", 96.0, INK).center().spaced(6.0),
        );
        photo(d, p, (60.0, 230.0, 470.0, 700.0), MINT_TINT);
        photo(d, p, (550.0, 230.0, 470.0, 700.0), CORAL_LIGHT);
        d.rect(p, (90.0, 860.0, 160.0, 50.0), INK, 25.0);
        d.text(
            p,
            (90.0, 870.0, 160.0),
            l.pick("ANTES", "BEFORE"),
            font("Montserrat", 22.0, WHITE).bold().center(),
        );
        d.rect(p, (830.0, 860.0, 160.0, 50.0), CORAL, 25.0);
        d.text(
            p,
            (830.0, 870.0, 160.0),
            l.pick("DESPUÉS", "AFTER"),
            font("Montserrat", 22.0, WHITE).bold().center(),
        );
        d.text(
            p,
            (60.0, 980.0, 960.0),
            "@tumarca",
            font("Inter", 26.0, SLATE).center(),
        );
    });
    d.document()
}

fn new_product(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(MINT_TINT), |d, p| {
        d.ellipse(p, (-120.0, -120.0, 420.0, 420.0), MINT, 0.6);
        d.ring(p, (270.0, 150.0, 540.0, 540.0), INK, 4.0);
        circle_photo(d, p, (290.0, 170.0, 500.0, 500.0), MINT);
        d.text(
            p,
            (60.0, 740.0, 960.0),
            l.pick("NUEVO", "NEW"),
            font("Montserrat", 34.0, MINT_DARK)
                .bold()
                .center()
                .spaced(10.0),
        );
        d.text(
            p,
            (60.0, 790.0, 960.0),
            l.pick("Velas de soja", "Soy candles"),
            font("DM Serif Display", 90.0, INK).center(),
        );
        d.burst(p, (820.0, 120.0, 200.0, 200.0), CORAL, 14, 0.4);
        d.text(
            p,
            (820.0, 190.0, 200.0),
            "12 €",
            font("Archivo Black", 40.0, WHITE).center(),
        );
    });
    d.document()
}

fn bold_quote(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(CORAL), |d, p| {
        // The quotes are a shape of the panel, not a letter (spec 098).
        d.piece(p, (72.0, 130.0, 106.0), "pieces.quote_cut", INK);
        d.text(
            p,
            (90.0, 340.0, 900.0),
            l.pick(
                "HECHO\nES MEJOR\nQUE PERFECTO",
                "DONE\nIS BETTER\nTHAN PERFECT",
            ),
            font("Archivo Black", 110.0, WHITE).line(0.85),
        );
        d.rect(p, (90.0, 880.0, 120.0, 8.0), INK, 4.0);
        d.text(
            p,
            (90.0, 910.0, 900.0),
            "@tumarca",
            font("Inter", 30.0, INK).bold(),
        );
    });
    d.document()
}

fn live_event(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(INK), |d, p| {
        d.ring(p, (620.0, -160.0, 640.0, 640.0), MINT, 6.0);
        d.ring(p, (700.0, -80.0, 480.0, 480.0), CORAL, 6.0);
        d.rect(p, (80.0, 100.0, 220.0, 70.0), CORAL, 35.0);
        d.text(
            p,
            (80.0, 116.0, 220.0),
            l.pick("EN VIVO", "LIVE"),
            font("Montserrat", 30.0, WHITE).bold().center().spaced(4.0),
        );
        d.text(
            p,
            (80.0, 260.0, 920.0),
            l.pick(
                "Charla:\ncómo empezar\ntu marca",
                "Talk:\nhow to start\nyour brand",
            ),
            font("Poppins", 92.0, WHITE).bold().line(1.0),
        );
        let blocks = [
            (l.pick("JUEVES", "THURSDAY"), "18"),
            (l.pick("HORA", "TIME"), "19:00"),
        ];
        for (i, (label, value)) in blocks.iter().enumerate() {
            let x = 80.0 + i as f64 * 330.0;
            d.rect(p, (x, 780.0, 300.0, 200.0), INK_LIGHT, 24.0);
            d.text(
                p,
                (x, 800.0, 300.0),
                label,
                font("Montserrat", 26.0, MINT).bold().center().spaced(4.0),
            );
            d.text(
                p,
                (x, 850.0, 300.0),
                value,
                font("Archivo Black", 80.0, WHITE).center(),
            );
        }
    });
    d.document()
}

fn recipe(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(CANVAS), |d, p| {
        photo(d, p, (0.0, 0.0, 1080.0, 520.0), CORAL_LIGHT);
        d.rect(p, (80.0, 430.0, 920.0, 580.0), WHITE, 32.0);
        d.text(
            p,
            (130.0, 470.0, 820.0),
            l.pick("Tostadas de aguacate", "Avocado toast"),
            font("DM Serif Display", 66.0, INK),
        );
        d.text(
            p,
            (130.0, 580.0, 820.0),
            l.pick("10 MIN  ·  2 PERSONAS", "10 MIN  ·  SERVES 2"),
            font("Montserrat", 24.0, MINT_DARK).bold().spaced(4.0),
        );
        let items = [
            l.pick("2 rebanadas de pan", "2 slices of bread"),
            l.pick("1 aguacate maduro", "1 ripe avocado"),
            l.pick("Limón, sal y pimienta", "Lemon, salt and pepper"),
            l.pick("Semillas al gusto", "Seeds to taste"),
        ];
        for (i, item) in items.iter().enumerate() {
            let y = 660.0 + i as f64 * 74.0;
            d.ellipse(p, (130.0, y + 8.0, 20.0, 20.0), CORAL, 1.0);
            d.text(p, (170.0, y, 780.0), item, font("Inter", 34.0, INK));
        }
    });
    d.document()
}

fn flash_sale(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(MINT), |d, p| {
        d.burst(p, (190.0, 150.0, 700.0, 700.0), CORAL, 20, 0.42);
        d.text(
            p,
            (190.0, 300.0, 700.0),
            l.pick("SOLO HOY", "TODAY ONLY"),
            font("Montserrat", 40.0, WHITE).bold().center().spaced(6.0),
        );
        d.text(
            p,
            (190.0, 370.0, 700.0),
            "-40%",
            font("Archivo Black", 170.0, WHITE).center(),
        );
        d.text(
            p,
            (190.0, 600.0, 700.0),
            l.pick("en toda la tienda", "on everything"),
            font("Montserrat", 36.0, WHITE).center(),
        );
        d.text(
            p,
            (60.0, 930.0, 960.0),
            l.pick("Usa el código  FLASH40", "Use code  FLASH40"),
            font("Montserrat", 34.0, INK).bold().center().spaced(2.0),
        );
    });
    d.document()
}

fn testimonial(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(MINT_PALE), |d, p| {
        d.rect(p, (80.0, 160.0, 920.0, 760.0), WHITE, 40.0);
        circle_photo(d, p, (430.0, 90.0, 220.0, 220.0), MINT_TINT);
        for i in 0..5 {
            d.star(p, (360.0 + f64::from(i) * 76.0, 350.0, 60.0, 60.0), CORAL);
        }
        d.text(
            p,
            (150.0, 460.0, 780.0),
            l.pick(
                "«Me encantó el resultado. Rápido, cercano y con mucho gusto por el detalle.»",
                "\"I loved the result. Fast, friendly and with real care for detail.\"",
            ),
            font("DM Serif Display", 48.0, INK).center().line(1.2),
        );
        d.text(
            p,
            (150.0, 800.0, 780.0),
            l.pick("María Torres · Clienta", "María Torres · Client"),
            font("Montserrat", 28.0, SLATE).bold().center(),
        );
    });
    d.document()
}

fn opening_hours(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(INK), |d, p| {
        d.text(
            p,
            (80.0, 90.0, 920.0),
            l.pick("HORARIO", "OPENING HOURS"),
            font("Bebas Neue", 130.0, CANVAS).spaced(6.0),
        );
        let rows = [
            (
                l.pick("Lunes a viernes", "Monday to Friday"),
                "9:00 – 20:00",
            ),
            (l.pick("Sábado", "Saturday"), "10:00 – 14:00"),
            (l.pick("Domingo", "Sunday"), l.pick("Cerrado", "Closed")),
        ];
        for (i, (day, hours)) in rows.iter().enumerate() {
            let y = 340.0 + i as f64 * 170.0;
            d.rect(p, (80.0, y + 120.0, 920.0, 2.0), INK_LIGHT, 0.0);
            d.text(
                p,
                (80.0, y + 30.0, 520.0),
                day,
                font("Montserrat", 40.0, CANVAS),
            );
            d.text(
                p,
                (560.0, y + 30.0, 440.0),
                hours,
                font("Montserrat", 40.0, MINT).bold().right(),
            );
        }
        d.text(
            p,
            (80.0, 930.0, 920.0),
            l.pick(
                "Calle Mayor 12 · Te esperamos",
                "12 Main Street · See you soon",
            ),
            font("Inter", 28.0, MIST),
        );
    });
    d.document()
}

fn podcast(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(SQUARE, solid(CORAL_LIGHT), |d, p| {
        d.rect(p, (0.0, 700.0, 1080.0, 380.0), INK, 0.0);
        d.ring(p, (570.0, 120.0, 460.0, 460.0), INK, 6.0);
        circle_photo(d, p, (590.0, 140.0, 420.0, 420.0), MINT_TINT);
        d.text(
            p,
            (70.0, 120.0, 520.0),
            "PODCAST",
            font("Montserrat", 34.0, INK).bold().spaced(8.0),
        );
        d.text(
            p,
            (70.0, 180.0, 520.0),
            l.pick("Ideas\nque\nsuman", "Ideas\nthat\nadd up"),
            font("Archivo Black", 100.0, INK).line(0.85),
        );
        d.text(
            p,
            (70.0, 760.0, 940.0),
            l.pick("EPISODIO 12", "EPISODE 12"),
            font("Montserrat", 30.0, CORAL_LIGHT).bold().spaced(6.0),
        );
        d.text(
            p,
            (70.0, 820.0, 940.0),
            l.pick("Cómo diseñar sin complicarte", "Design without the hassle"),
            font("Poppins", 52.0, WHITE).bold(),
        );
        d.text(
            p,
            (70.0, 940.0, 940.0),
            l.pick(
                "Disponible en todas las plataformas",
                "Out now on every platform",
            ),
            font("Inter", 28.0, MIST),
        );
    });
    d.document()
}

// ---------------------------------------------- business cards (85 × 55 mm)

const BCARD: (f64, f64, &str) = (85.0, 55.0, "mm");
const BW: f64 = 85.0 * MM;
const BH: f64 = 55.0 * MM;

fn card_minimal_line(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(BCARD, solid(CANVAS), |d, p| {
        d.text(
            p,
            (24.0, 50.0, BW - 48.0),
            "Daniela Ruiz",
            font("Cormorant Garamond", 30.0, INK).center(),
        );
        d.rect(p, (BW / 2.0 - 30.0, 98.0, 60.0, 1.5), CORAL, 0.0);
        d.text(
            p,
            (24.0, 110.0, BW - 48.0),
            l.pick("ARQUITECTA", "ARCHITECT"),
            font("Montserrat", 9.0, SLATE).center().spaced(4.0),
        );
        d.text(
            p,
            (24.0, 150.0, BW - 48.0),
            "+34 600 000 000  ·  daniela@estudio.com",
            font("Montserrat", 8.0, INK).center(),
        );
    });
    d.page(BCARD, solid(INK), |d, p| {
        d.ring(
            p,
            (BW / 2.0 - 34.0, BH / 2.0 - 34.0, 68.0, 68.0),
            CORAL,
            2.0,
        );
        d.text(
            p,
            (BW / 2.0 - 34.0, BH / 2.0 - 17.0, 68.0),
            "DR",
            font("Cormorant Garamond", 26.0, CANVAS).center(),
        );
    });
    d.document()
}

fn card_split(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(BCARD, solid(WHITE), |d, p| {
        d.rect(p, (0.0, 0.0, BW, 60.0), CORAL, 0.0);
        d.text(
            p,
            (20.0, 18.0, BW - 40.0),
            l.pick("PANADERÍA LA ESPIGA", "THE WHEAT BAKERY"),
            font("Montserrat", 12.0, WHITE).bold().spaced(3.0),
        );
        d.text(
            p,
            (20.0, 82.0, BW - 40.0),
            "Tomás Herrera",
            font("Poppins", 20.0, INK).bold(),
        );
        d.text(
            p,
            (20.0, 112.0, BW - 40.0),
            l.pick("Maestro panadero", "Master baker"),
            font("Poppins", 10.0, CORAL),
        );
        d.text(
            p,
            (20.0, 145.0, BW - 40.0),
            "+34 600 000 000\nhola@laespiga.com",
            font("Inter", 9.0, SLATE).line(1.5),
        );
    });
    d.page(BCARD, pattern("dots", CORAL_LIGHT, CORAL), |d, p| {
        d.rect(
            p,
            (BW / 2.0 - 90.0, BH / 2.0 - 30.0, 180.0, 60.0),
            WHITE,
            30.0,
        );
        d.text(
            p,
            (BW / 2.0 - 90.0, BH / 2.0 - 12.0, 180.0),
            l.pick("La Espiga", "The Wheat"),
            font("Pacifico", 18.0, CORAL).center(),
        );
    });
    d.document()
}

fn card_photo(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(BCARD, solid(MINT_PALE), |d, p| {
        circle_photo(d, p, (22.0, BH / 2.0 - 50.0, 100.0, 100.0), MINT_TINT);
        d.text(
            p,
            (140.0, 50.0, BW - 160.0),
            "Lucía Romero",
            font("Manrope", 17.0, INK).bold(),
        );
        d.text(
            p,
            (140.0, 76.0, BW - 160.0),
            l.pick("Fotógrafa de bodas", "Wedding photographer"),
            font("Inter", 9.0, MINT_DARK),
        );
        d.rect(p, (140.0, 102.0, 40.0, 2.0), MINT_DARK, 0.0);
        d.text(
            p,
            (140.0, 114.0, BW - 160.0),
            "+34 600 000 000\nlucia@foto.com\n@luciafoto",
            font("Inter", 8.5, SLATE).line(1.5),
        );
    });
    d.document()
}

fn card_bold(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(BCARD, solid(INK), |d, p| {
        d.burst(p, (BW - 110.0, -40.0, 160.0, 160.0), MINT, 12, 0.35);
        d.text(
            p,
            (20.0, 40.0, BW - 40.0),
            l.pick("ESTUDIO\nNÓMADA", "NOMAD\nSTUDIO"),
            font("Archivo Black", 26.0, WHITE).line(0.85),
        );
        d.text(
            p,
            (20.0, 125.0, BW - 40.0),
            l.pick("Diseño y desarrollo web", "Web design and development"),
            font("Inter", 9.0, MINT),
        );
        d.text(
            p,
            (20.0, 160.0, BW - 40.0),
            "nomada.studio  ·  +34 600 000 000",
            font("Inter", 8.0, MIST),
        );
    });
    d.page(BCARD, solid(MINT), |d, p| {
        d.text(
            p,
            (20.0, BH / 2.0 - 20.0, BW - 40.0),
            "N.",
            font("Archivo Black", 40.0, INK).center(),
        );
    });
    d.document()
}

// ---------------------------------------- certificates (A4 horizontal)

const A4H: (f64, f64, &str) = (297.0, 210.0, "mm");
const AW: f64 = 297.0 * MM;
const AH: f64 = 210.0 * MM;

fn diploma_classic(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4H, solid(CANVAS), |d, p| {
        d.rect(p, (0.0, 0.0, AW, 70.0), INK, 0.0);
        d.rect(p, (0.0, AH - 70.0, AW, 70.0), INK, 0.0);
        d.rect(p, (0.0, 70.0, AW, 6.0), CORAL, 0.0);
        d.rect(p, (0.0, AH - 76.0, AW, 6.0), CORAL, 0.0);
        d.text(
            p,
            (80.0, 130.0, AW - 160.0),
            l.pick("DIPLOMA", "DIPLOMA"),
            font("Cormorant Garamond", 70.0, INK)
                .bold()
                .center()
                .spaced(12.0),
        );
        d.text(
            p,
            (80.0, 240.0, AW - 160.0),
            l.pick(
                "Se otorga el presente diploma a",
                "This diploma is awarded to",
            ),
            font("Montserrat", 18.0, SLATE).center(),
        );
        d.text(
            p,
            (80.0, 290.0, AW - 160.0),
            "Sofía Martínez",
            font("Great Vibes", 80.0, INK).center(),
        );
        d.rect(p, (AW * 0.25, 410.0, AW * 0.5, 1.5), CORAL, 0.0);
        d.text(
            p,
            (140.0, 430.0, AW - 280.0),
            l.pick(
                "por haber completado el curso de Diseño Gráfico con excelentes resultados.",
                "for completing the Graphic Design course with excellent results.",
            ),
            font("Montserrat", 16.0, SLATE).center().line(1.4),
        );
        for (i, (who, role)) in [
            ("Ana López", l.pick("Directora", "Director")),
            ("Juan Pérez", l.pick("Profesor", "Teacher")),
        ]
        .iter()
        .enumerate()
        {
            let x = if i == 0 { 160.0 } else { AW - 420.0 };
            d.rect(p, (x, 600.0, 260.0, 1.5), INK, 0.0);
            d.text(
                p,
                (x, 610.0, 260.0),
                who,
                font("Montserrat", 14.0, INK).bold().center(),
            );
            d.text(
                p,
                (x, 632.0, 260.0),
                role,
                font("Montserrat", 12.0, SLATE).center(),
            );
        }
    });
    d.document()
}

fn certificate_modern(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4H, solid(WHITE), |d, p| {
        d.rect(p, (0.0, 0.0, 300.0, AH), MINT_DARK, 0.0);
        d.polygon(
            p,
            (220.0, 0.0, 160.0, AH),
            MINT_DARK,
            &[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
            1.0,
        );
        d.burst(p, (70.0, 300.0, 160.0, 160.0), MINT, 24, 0.42);
        d.ellipse(p, (100.0, 330.0, 100.0, 100.0), MINT_DARK, 1.0);
        d.star(p, (125.0, 355.0, 50.0, 50.0), WHITE);
        d.text(
            p,
            (420.0, 120.0, AW - 500.0),
            l.pick("CERTIFICADO", "CERTIFICATE"),
            font("Montserrat", 54.0, INK).bold().spaced(6.0),
        );
        d.text(
            p,
            (420.0, 200.0, AW - 500.0),
            l.pick("DE PARTICIPACIÓN", "OF PARTICIPATION"),
            font("Montserrat", 20.0, MINT_DARK).bold().spaced(6.0),
        );
        d.text(
            p,
            (420.0, 290.0, AW - 500.0),
            l.pick("Otorgado a", "Presented to"),
            font("Montserrat", 16.0, SLATE),
        );
        d.text(
            p,
            (420.0, 320.0, AW - 500.0),
            "Carlos Vega",
            font("DM Serif Display", 68.0, INK),
        );
        d.text(
            p,
            (420.0, 430.0, AW - 520.0),
            l.pick(
                "Por su participación en el Congreso de Innovación 2026, celebrado en Madrid.",
                "For taking part in the 2026 Innovation Congress, held in Madrid.",
            ),
            font("Montserrat", 15.0, SLATE).line(1.5),
        );
        d.rect(p, (420.0, 620.0, 240.0, 1.5), INK, 0.0);
        d.text(
            p,
            (420.0, 630.0, 240.0),
            l.pick("Firma", "Signature"),
            font("Montserrat", 12.0, SLATE),
        );
        d.rect(p, (720.0, 620.0, 240.0, 1.5), INK, 0.0);
        d.text(
            p,
            (720.0, 630.0, 240.0),
            l.pick("Fecha", "Date"),
            font("Montserrat", 12.0, SLATE),
        );
    });
    d.document()
}

fn kids_diploma(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4H, solid(MINT_PALE), |d, p| {
        for (x, y, c) in [
            (40.0, 40.0, CORAL),
            (AW - 120.0, 50.0, MINT),
            (60.0, AH - 120.0, MINT),
            (AW - 140.0, AH - 130.0, CORAL),
        ] {
            d.star(p, (x, y, 80.0, 80.0), c);
        }
        d.frame(p, (30.0, 30.0, AW - 60.0, AH - 60.0), CORAL, 6.0, 30.0);
        d.text(
            p,
            (120.0, 110.0, AW - 240.0),
            l.pick("¡Diploma al\nmejor lector!", "Best Reader\nAward!"),
            font("Chewy", 72.0, CORAL).center().line(0.95),
        );
        d.text(
            p,
            (120.0, 330.0, AW - 240.0),
            l.pick("Para", "For"),
            font("Chewy", 26.0, INK).center(),
        );
        d.text(
            p,
            (120.0, 370.0, AW - 240.0),
            "Mateo García",
            font("Chewy", 64.0, INK).center(),
        );
        d.text(
            p,
            (160.0, 480.0, AW - 320.0),
            l.pick(
                "Por leer 20 libros este curso. ¡Sigue soñando con cada página!",
                "For reading 20 books this year. Keep dreaming with every page!",
            ),
            font("Poppins", 18.0, SLATE).center().line(1.4),
        );
        d.text(
            p,
            (120.0, 620.0, AW - 240.0),
            l.pick("Seño Carmen · Junio 2026", "Miss Carmen · June 2026"),
            font("Chewy", 22.0, MINT_DARK).center(),
        );
    });
    d.document()
}

fn award_gold(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4H, solid(INK), |d, p| {
        d.frame(p, (36.0, 36.0, AW - 72.0, AH - 72.0), CORAL, 3.0, 0.0);
        d.frame(p, (52.0, 52.0, AW - 104.0, AH - 104.0), CORAL, 1.0, 0.0);
        d.burst(p, (AW / 2.0 - 60.0, 90.0, 120.0, 120.0), CORAL, 16, 0.35);
        d.text(
            p,
            (100.0, 240.0, AW - 200.0),
            l.pick("RECONOCIMIENTO", "AWARD OF EXCELLENCE"),
            font("Cormorant Garamond", 54.0, CANVAS)
                .bold()
                .center()
                .spaced(8.0),
        );
        d.text(
            p,
            (100.0, 330.0, AW - 200.0),
            l.pick("A la empleada del año", "Employee of the year"),
            font("Montserrat", 18.0, CORAL).center().spaced(4.0),
        );
        d.text(
            p,
            (100.0, 380.0, AW - 200.0),
            "Valeria Santos",
            font("Great Vibes", 78.0, CANVAS).center(),
        );
        d.text(
            p,
            (160.0, 500.0, AW - 320.0),
            l.pick(
                "Por su liderazgo, su compromiso y su manera de inspirar al equipo.",
                "For her leadership, her commitment and the way she inspires the team.",
            ),
            font("Montserrat", 15.0, MIST).center().line(1.4),
        );
        d.text(
            p,
            (100.0, 640.0, AW - 200.0),
            l.pick("Diciembre 2026", "December 2026"),
            font("Montserrat", 14.0, CORAL).center().spaced(3.0),
        );
    });
    d.document()
}

// ------------------------------------------------- resumes (A4 vertical)

const A4V: (f64, f64, &str) = (210.0, 297.0, "mm");
const VW: f64 = 210.0 * MM;
const VH: f64 = 297.0 * MM;

/// The sections of a résumé, in `l`.
fn resume_sections(l: Lang) -> [(&'static str, &'static str); 3] {
    [
        (l.pick("PERFIL", "PROFILE"), l.pick("Profesional organizada y curiosa, con seis años de experiencia en equipos de producto.", "An organised, curious professional with six years of experience in product teams.")),
        (l.pick("EXPERIENCIA", "EXPERIENCE"), l.pick("Empresa Uno · 2022 – hoy\nCoordina proyectos y equipos.\n\nEmpresa Dos · 2019 – 2022\nAnalista de datos.", "Company One · 2022 – now\nRuns projects and teams.\n\nCompany Two · 2019 – 2022\nData analyst.")),
        (l.pick("FORMACIÓN", "EDUCATION"), l.pick("Máster en Gestión · Universidad Central", "MSc in Management · Central University")),
    ]
}

fn resume_header(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4V, solid(WHITE), |d, p| {
        d.rect(p, (0.0, 0.0, VW, 230.0), INK, 0.0);
        circle_photo(d, p, (60.0, 50.0, 130.0, 130.0), INK_LIGHT);
        d.text(
            p,
            (220.0, 70.0, VW - 260.0),
            "Andrea Molina",
            font("Poppins", 40.0, WHITE).bold(),
        );
        d.text(
            p,
            (220.0, 130.0, VW - 260.0),
            l.pick("Gerente de proyectos", "Project manager"),
            font("Poppins", 17.0, CORAL),
        );
        d.text(
            p,
            (220.0, 165.0, VW - 260.0),
            "+34 600 000 000  ·  andrea@correo.com  ·  Madrid",
            font("Inter", 11.0, MIST),
        );
        let mut y = 280.0;
        for (title, body) in resume_sections(l) {
            d.text(
                p,
                (60.0, y, VW - 120.0),
                title,
                font("Poppins", 15.0, CORAL).bold().spaced(3.0),
            );
            d.rect(p, (60.0, y + 28.0, VW - 120.0, 1.0), BORDER, 0.0);
            d.text(
                p,
                (60.0, y + 44.0, VW - 120.0),
                body,
                font("Inter", 13.0, SLATE).line(1.6),
            );
            y += 90.0 + body.lines().count() as f64 * 22.0;
        }
        d.text(
            p,
            (60.0, y, VW - 120.0),
            l.pick("HABILIDADES", "SKILLS"),
            font("Poppins", 15.0, CORAL).bold().spaced(3.0),
        );
        for (i, skill) in [
            l.pick("Liderazgo", "Leadership"),
            l.pick("Planificación", "Planning"),
            "Excel",
            l.pick("Comunicación", "Communication"),
        ]
        .iter()
        .enumerate()
        {
            let x = 60.0 + i as f64 * 165.0;
            d.rect(p, (x, y + 40.0, 150.0, 34.0), MINT_TINT, 17.0);
            d.text(
                p,
                (x, y + 48.0, 150.0),
                skill,
                font("Inter", 12.0, INK).center(),
            );
        }
    });
    d.document()
}

fn resume_two_columns(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4V, solid(WHITE), |d, p| {
        d.rect(p, (VW - 260.0, 0.0, 260.0, VH), MINT_PALE, 0.0);
        d.text(
            p,
            (50.0, 70.0, VW - 340.0),
            "Pablo\nIglesias",
            font("DM Serif Display", 52.0, INK).line(0.95),
        );
        d.text(
            p,
            (50.0, 220.0, VW - 340.0),
            l.pick("Desarrollador de software", "Software developer"),
            font("Inter", 15.0, MINT_DARK).bold(),
        );
        let mut y = 290.0;
        for (title, body) in resume_sections(l) {
            d.text(
                p,
                (50.0, y, VW - 340.0),
                title,
                font("Inter", 13.0, INK).bold().spaced(3.0),
            );
            d.text(
                p,
                (50.0, y + 30.0, VW - 340.0),
                body,
                font("Inter", 12.5, SLATE).line(1.6),
            );
            y += 80.0 + body.lines().count() as f64 * 22.0;
        }
        let x = VW - 230.0;
        rounded_photo(d, p, (x, 60.0, 200.0, 220.0), MINT_TINT, 16.0);
        d.text(
            p,
            (x, 320.0, 200.0),
            l.pick("CONTACTO", "CONTACT"),
            font("Inter", 13.0, INK).bold().spaced(3.0),
        );
        d.text(
            p,
            (x, 350.0, 200.0),
            "+34 600 000 000\npablo@correo.com\ngithub.com/pablo",
            font("Inter", 11.5, SLATE).line(1.7),
        );
        d.text(
            p,
            (x, 470.0, 200.0),
            l.pick("IDIOMAS", "LANGUAGES"),
            font("Inter", 13.0, INK).bold().spaced(3.0),
        );
        d.text(
            p,
            (x, 500.0, 200.0),
            l.pick(
                "Español · nativo\nInglés · avanzado",
                "Spanish · native\nEnglish · fluent",
            ),
            font("Inter", 11.5, SLATE).line(1.7),
        );
        d.text(
            p,
            (x, 600.0, 200.0),
            l.pick("HERRAMIENTAS", "TOOLS"),
            font("Inter", 13.0, INK).bold().spaced(3.0),
        );
        for (i, (tool, level)) in [("Rust", 0.9), ("TypeScript", 0.8), ("SQL", 0.7)]
            .iter()
            .enumerate()
        {
            let y = 640.0 + i as f64 * 50.0;
            d.text(p, (x, y, 200.0), tool, font("Inter", 11.5, SLATE));
            d.rect(p, (x, y + 22.0, 190.0, 6.0), BORDER, 3.0);
            d.rect(p, (x, y + 22.0, 190.0 * level, 6.0), MINT_DARK, 3.0);
        }
    });
    d.document()
}

fn resume_elegant(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4V, solid(CANVAS), |d, p| {
        d.text(
            p,
            (60.0, 70.0, VW - 120.0),
            "ELENA  CASTRO",
            font("Cormorant Garamond", 46.0, INK).center().spaced(8.0),
        );
        d.text(
            p,
            (60.0, 140.0, VW - 120.0),
            l.pick("ABOGADA", "LAWYER"),
            font("Montserrat", 13.0, CORAL).center().spaced(10.0),
        );
        d.rect(p, (60.0, 180.0, VW - 120.0, 1.0), INK, 0.0);
        d.text(
            p,
            (60.0, 192.0, VW - 120.0),
            "+34 600 000 000   ·   elena@despacho.com   ·   Sevilla",
            font("Montserrat", 11.0, SLATE).center(),
        );
        d.rect(p, (60.0, 222.0, VW - 120.0, 1.0), INK, 0.0);
        let mut y = 270.0;
        for (title, body) in resume_sections(l) {
            d.text(
                p,
                (60.0, y, 180.0),
                title,
                font("Cormorant Garamond", 18.0, INK).bold().spaced(2.0),
            );
            d.text(
                p,
                (250.0, y + 2.0, VW - 310.0),
                body,
                font("Montserrat", 12.0, SLATE).line(1.6),
            );
            y += 70.0 + body.lines().count() as f64 * 22.0;
        }
    });
    d.document()
}

// ------------------------------------------------- flyers (A4 vertical)

fn flyer_yoga(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4V, solid(MINT_PALE), |d, p| {
        d.ellipse(p, (VW / 2.0 - 300.0, 120.0, 600.0, 600.0), MINT_TINT, 1.0);
        circle_photo(d, p, (VW / 2.0 - 230.0, 190.0, 460.0, 460.0), MINT);
        d.text(
            p,
            (60.0, 720.0, VW - 120.0),
            l.pick("Clases de yoga", "Yoga classes"),
            font("DM Serif Display", 64.0, INK).center(),
        );
        d.text(
            p,
            (60.0, 820.0, VW - 120.0),
            l.pick(
                "Para todos los niveles · Primera clase gratis",
                "All levels welcome · First class free",
            ),
            font("Montserrat", 17.0, MINT_DARK).center(),
        );
        d.rect(p, (VW / 2.0 - 160.0, 900.0, 320.0, 60.0), INK, 30.0);
        d.text(
            p,
            (VW / 2.0 - 160.0, 916.0, 320.0),
            l.pick("Lun y Mié · 19:00", "Mon & Wed · 7 PM"),
            font("Montserrat", 17.0, WHITE).bold().center(),
        );
        d.text(
            p,
            (60.0, 1010.0, VW - 120.0),
            "www.estudioyoga.com",
            font("Montserrat", 14.0, SLATE).center(),
        );
    });
    d.document()
}

fn flyer_real_estate(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4V, solid(WHITE), |d, p| {
        photo(d, p, (0.0, 0.0, VW, 560.0), MINT_TINT);
        d.rect(p, (0.0, 520.0, 420.0, 90.0), CORAL, 0.0);
        d.text(
            p,
            (40.0, 540.0, 360.0),
            l.pick("EN VENTA", "FOR SALE"),
            font("Montserrat", 34.0, WHITE).bold().spaced(6.0),
        );
        d.text(
            p,
            (40.0, 650.0, VW - 80.0),
            l.pick(
                "Casa con jardín en el centro",
                "House with a garden downtown",
            ),
            font("Poppins", 34.0, INK).bold(),
        );
        photo(d, p, (40.0, 730.0, 220.0, 160.0), MINT_TINT);
        photo(d, p, (287.0, 730.0, 220.0, 160.0), CORAL_LIGHT);
        photo(d, p, (534.0, 730.0, 220.0, 160.0), MIST);
        let facts = [
            l.pick("3 habitaciones", "3 bedrooms"),
            l.pick("2 baños", "2 bathrooms"),
            "140 m²",
        ];
        for (i, fact) in facts.iter().enumerate() {
            d.text(
                p,
                (40.0 + i as f64 * 247.0, 920.0, 220.0),
                fact,
                font("Montserrat", 16.0, SLATE).bold().center(),
            );
        }
        d.rect(p, (0.0, VH - 120.0, VW, 120.0), INK, 0.0);
        d.text(
            p,
            (40.0, VH - 85.0, VW - 80.0),
            "320.000 €   ·   +34 600 000 000",
            font("Montserrat", 24.0, WHITE).bold().center(),
        );
    });
    d.document()
}

fn flyer_workshop(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4V, solid(CORAL), |d, p| {
        d.ring(p, (-200.0, -200.0, 600.0, 600.0), CORAL_LIGHT, 40.0);
        d.ring(p, (VW - 300.0, VH - 360.0, 600.0, 600.0), CORAL_LIGHT, 40.0);
        d.text(
            p,
            (70.0, 220.0, VW - 140.0),
            l.pick("TALLER", "WORKSHOP"),
            font("Bebas Neue", 70.0, INK).spaced(10.0),
        );
        d.text(
            p,
            (70.0, 300.0, VW - 140.0),
            l.pick("Cerámica\npara\nprincipiantes", "Pottery\nfor\nbeginners"),
            font("Archivo Black", 76.0, WHITE).line(0.9),
        );
        d.rect(p, (70.0, 720.0, VW - 140.0, 2.0), INK, 0.0);
        let rows = [
            (
                l.pick("CUÁNDO", "WHEN"),
                l.pick("Sábado 7 de marzo · 10:00", "Saturday, March 7 · 10 AM"),
            ),
            (
                l.pick("DÓNDE", "WHERE"),
                l.pick("Taller Arcilla, Calle Sol 4", "Clay Studio, 4 Sun Street"),
            ),
            (
                l.pick("PRECIO", "PRICE"),
                l.pick("35 € con materiales", "€35 materials included"),
            ),
        ];
        for (i, (k, v)) in rows.iter().enumerate() {
            let y = 750.0 + i as f64 * 70.0;
            d.text(
                p,
                (70.0, y, 180.0),
                k,
                font("Montserrat", 16.0, INK).bold().spaced(3.0),
            );
            d.text(
                p,
                (260.0, y, VW - 330.0),
                v,
                font("Montserrat", 18.0, WHITE),
            );
        }
    });
    d.document()
}

fn flyer_restaurant(l: Lang) -> Document {
    let mut d = Doc::new();
    d.page(A4V, solid(INK), |d, p| {
        d.text(
            p,
            (60.0, 80.0, VW - 120.0),
            l.pick("Nuevo menú", "New menu"),
            font("Great Vibes", 70.0, MINT).center(),
        );
        d.text(
            p,
            (60.0, 180.0, VW - 120.0),
            "TRATTORIA ROMA",
            font("Cormorant Garamond", 52.0, CANVAS)
                .bold()
                .center()
                .spaced(8.0),
        );
        circle_photo(d, p, (VW / 2.0 - 200.0, 290.0, 400.0, 400.0), INK_LIGHT);
        let dishes = [
            (l.pick("Pasta fresca al pesto", "Fresh pesto pasta"), "12 €"),
            (l.pick("Pizza margarita", "Margherita pizza"), "10 €"),
            (l.pick("Tiramisú de la casa", "House tiramisu"), "6 €"),
        ];
        for (i, (dish, price)) in dishes.iter().enumerate() {
            let y = 760.0 + i as f64 * 70.0;
            d.text(
                p,
                (120.0, y, VW - 360.0),
                dish,
                font("Cormorant Garamond", 26.0, CANVAS),
            );
            d.text(
                p,
                (VW - 240.0, y, 120.0),
                price,
                font("Cormorant Garamond", 26.0, MINT).bold().right(),
            );
            d.rect(p, (120.0, y + 44.0, VW - 240.0, 1.0), INK_LIGHT, 0.0);
        }
        d.text(
            p,
            (60.0, 1020.0, VW - 120.0),
            l.pick("Reservas: +34 600 000 000", "Bookings: +34 600 000 000"),
            font("Montserrat", 15.0, MIST).center(),
        );
    });
    d.document()
}

const DESIGNS: &[Design] = &[
    Design {
        folder: "social",
        file: "corner-coffee",
        names: ("Café de barrio", "Corner coffee"),
        kind: "size.ig_square",
        page_format: "social",
        style: "warm",
        color: INK,
        tags: &["café", "coffee", "comida", "food"],
        draw: cafe,
    },
    Design {
        folder: "social",
        file: "tips-carousel",
        names: ("Consejos en carrusel", "Tips carousel"),
        kind: "size.ig_square",
        page_format: "social",
        style: "bold",
        color: INK,
        tags: &["consejos", "tips", "carrusel", "carousel"],
        draw: tips_carousel,
    },
    Design {
        folder: "social",
        file: "summer-sale",
        names: ("Gran venta de verano", "Big summer sale"),
        kind: "size.fb_post",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &["venta", "sale", "oferta", "descuento", "discount"],
        draw: sale,
    },
    Design {
        folder: "social",
        file: "quote-of-the-day",
        names: ("Frase del día", "Quote of the day"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "minimal",
        color: MINT,
        tags: &["frase", "quote", "cita"],
        draw: quote,
    },
    Design {
        folder: "social",
        file: "live-workshop",
        names: ("Taller en vivo", "Live workshop"),
        kind: "size.ig_story",
        page_format: "social",
        style: "vibrant",
        color: MINT_DARK,
        tags: &["evento", "event", "historia", "story", "taller"],
        draw: workshop_story,
    },
    Design {
        folder: "social",
        file: "we-are-hiring",
        names: ("Oferta de empleo", "We're hiring"),
        kind: "size.li_post",
        page_format: "social",
        style: "minimal",
        color: MINT,
        tags: &["empleo", "job", "trabajo", "hiring"],
        draw: product_post,
    },
    Design {
        folder: "video",
        file: "tricks-thumbnail",
        names: ("Miniatura de trucos", "Tricks thumbnail"),
        kind: "size.yt_thumb",
        page_format: "video_horizontal",
        style: "bold",
        color: MINT,
        tags: &["youtube", "miniatura", "thumbnail"],
        draw: thumbnail_video,
    },
    Design {
        folder: "presentations",
        file: "creative-project",
        names: ("Proyecto creativo", "Creative project"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "bold",
        color: INK,
        tags: &["propuesta", "proposal", "proyecto", "project"],
        draw: project_deck,
    },
    Design {
        folder: "presentations",
        file: "annual-report",
        names: ("Informe anual", "Annual report"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "minimal",
        color: MINT_DARK,
        tags: &["informe", "report", "datos", "data"],
        draw: minimal_report,
    },
    Design {
        folder: "print",
        file: "jazz-night",
        names: ("Noche de jazz", "Jazz night"),
        kind: "size.flyer_a4",
        page_format: "print",
        style: "bold",
        color: CORAL,
        tags: &[
            "concierto",
            "concert",
            "música",
            "music",
            "afiche",
            "poster",
        ],
        draw: concert_flyer,
    },
    Design {
        folder: "print",
        file: "birthday-invitation",
        names: ("Invitación de cumpleaños", "Birthday invitation"),
        kind: "size.invitation",
        page_format: "print",
        style: "playful",
        color: CORAL,
        tags: &["cumpleaños", "birthday", "fiesta", "party"],
        draw: birthday,
    },
    Design {
        folder: "print",
        file: "business-card",
        names: ("Tarjeta de presentación", "Business card"),
        kind: "size.business_card",
        page_format: "print",
        style: "minimal",
        color: INK,
        tags: &["tarjeta", "card", "contacto", "contact"],
        draw: business_card,
    },
    Design {
        folder: "print",
        file: "certificate",
        names: (
            "Certificado de reconocimiento",
            "Certificate of recognition",
        ),
        kind: "size.a4_h",
        page_format: "print",
        style: "classic",
        color: MINT_DARK,
        tags: &["certificado", "certificate", "diploma"],
        draw: certificate,
    },
    Design {
        folder: "documents",
        file: "resume",
        names: ("Hoja de vida", "Resume"),
        kind: "size.resume",
        page_format: "print",
        style: "minimal",
        color: MINT,
        tags: &["cv", "currículum", "resume", "trabajo"],
        draw: resume,
    },
    Design {
        folder: "web",
        file: "newsletter",
        names: ("Boletín mensual", "Monthly newsletter"),
        kind: "size.email_header",
        page_format: "custom",
        style: "minimal",
        color: MINT,
        tags: &["correo", "email", "boletín", "newsletter"],
        draw: newsletter,
    },
    Design {
        folder: "web",
        file: "store-banner",
        names: ("Banner de tienda", "Store banner"),
        kind: "size.web_banner",
        page_format: "custom",
        style: "bold",
        color: INK,
        tags: &["banner", "tienda", "store", "web"],
        draw: web_banner,
    },
    Design {
        folder: "presentations",
        file: "brandbook",
        names: ("Brandbook", "Brandbook"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "bold",
        color: MINT_DARK,
        tags: &["marca", "brand", "identidad", "identity"],
        draw: brandbook,
    },
    Design {
        folder: "presentations",
        file: "brand-manual",
        names: ("Manual de marca", "Brand manual"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "minimal",
        color: CORAL,
        tags: &["marca", "brand", "valores", "values"],
        draw: brand_manual,
    },
    Design {
        folder: "presentations",
        file: "marketing-plan",
        names: ("Plan de marketing", "Marketing plan"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "bold",
        color: CORAL,
        tags: &["marketing", "redes", "social", "plan"],
        draw: marketing_plan,
    },
    Design {
        folder: "presentations",
        file: "color-psychology",
        names: ("Psicología del color", "Psychology of colour"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "playful",
        color: MINT,
        tags: &["color", "colour", "educación", "education"],
        draw: color_psychology,
    },
    Design {
        folder: "presentations",
        file: "creative-brief",
        names: ("Brief creativo", "Creative brief"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "bold",
        color: INK,
        tags: &["brief", "agencia", "agency", "campaña", "campaign"],
        draw: creative_brief,
    },
    Design {
        folder: "presentations",
        file: "final-project",
        names: ("Proyecto final", "Final project"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "vibrant",
        color: CORAL,
        tags: &["escuela", "school", "equipo", "team", "proyecto"],
        draw: final_project,
    },
    Design {
        folder: "presentations",
        file: "web-design",
        names: ("Diseño web", "Web design"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "bold",
        color: INK,
        tags: &["web", "diseño", "design", "clase", "class"],
        draw: web_design,
    },
    Design {
        folder: "presentations",
        file: "portfolio",
        names: ("Portafolio profesional", "Professional portfolio"),
        kind: "size.pres_16_9",
        page_format: "presentation",
        style: "minimal",
        color: INK,
        tags: &["portafolio", "portfolio", "fotografía", "photography"],
        draw: portfolio,
    },
    Design {
        folder: "print",
        file: "beauty-centre",
        names: ("Folleto centro de estética", "Beauty centre brochure"),
        kind: "size.trifold",
        page_format: "print",
        style: "classic",
        color: MINT_DARK,
        tags: &[
            "folleto",
            "brochure",
            "tríptico",
            "trifold",
            "estética",
            "beauty",
            "spa",
        ],
        draw: beauty_center,
    },
    Design {
        folder: "print",
        file: "plant-nursery",
        names: ("Folleto de vivero", "Plant nursery brochure"),
        kind: "size.trifold",
        page_format: "print",
        style: "playful",
        color: MINT,
        tags: &[
            "folleto",
            "brochure",
            "tríptico",
            "trifold",
            "plantas",
            "plants",
        ],
        draw: plant_nursery,
    },
    Design {
        folder: "print",
        file: "interior-studio",
        names: ("Folleto de interiorismo", "Interior design brochure"),
        kind: "size.trifold",
        page_format: "print",
        style: "classic",
        color: INK_LIGHT,
        tags: &[
            "folleto",
            "brochure",
            "tríptico",
            "trifold",
            "interiores",
            "interior",
            "hogar",
            "home",
        ],
        draw: interior_studio,
    },
    Design {
        folder: "print",
        file: "stationery-shop",
        names: ("Folleto de papelería", "Stationery shop brochure"),
        kind: "size.trifold",
        page_format: "print",
        style: "playful",
        color: CORAL,
        tags: &[
            "folleto",
            "brochure",
            "tríptico",
            "trifold",
            "escuela",
            "school",
        ],
        draw: stationery,
    },
    Design {
        folder: "print",
        file: "marketing-agency",
        names: (
            "Folleto de agencia de marketing",
            "Marketing agency brochure",
        ),
        kind: "size.trifold",
        page_format: "print",
        style: "minimal",
        color: MINT,
        tags: &[
            "folleto",
            "brochure",
            "tríptico",
            "trifold",
            "marketing",
            "agencia",
            "agency",
        ],
        draw: marketing_agency,
    },
    Design {
        folder: "print",
        file: "travel-agency",
        names: ("Folleto de agencia de viajes", "Travel agency brochure"),
        kind: "size.trifold",
        page_format: "print",
        style: "classic",
        color: MINT_DARK,
        tags: &[
            "folleto",
            "brochure",
            "tríptico",
            "trifold",
            "viajes",
            "travel",
            "turismo",
        ],
        draw: travel_agency,
    },
    Design {
        folder: "social",
        file: "new-hobbies",
        names: ("Mis nuevos hobbies", "My new hobbies"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "playful",
        color: CORAL,
        tags: &["instagram", "hobbies", "estilo de vida", "lifestyle"],
        draw: new_hobbies,
    },
    Design {
        folder: "social",
        file: "creative-routine",
        names: ("Mi rutina creativa", "My creative routine"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "bold",
        color: MINT,
        tags: &["instagram", "rutina", "routine", "trabajo", "work"],
        draw: creative_routine,
    },
    Design {
        folder: "social",
        file: "new-collection",
        names: ("Nueva colección", "New collection"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "classic",
        color: MINT_DARK,
        tags: &["instagram", "moda", "fashion", "tienda", "shop"],
        draw: new_collection,
    },
    Design {
        folder: "social",
        file: "month-in-numbers",
        names: ("Mi mes en números", "My month in numbers"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &["instagram", "resumen", "recap", "datos"],
        draw: month_in_numbers,
    },
    Design {
        folder: "social",
        file: "happy-birthday",
        names: ("Feliz cumpleaños", "Happy birthday"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "playful",
        color: CORAL,
        tags: &["instagram", "cumpleaños", "birthday", "fiesta", "party"],
        draw: happy_birthday,
    },
    Design {
        folder: "social",
        file: "launch-promo",
        names: ("Promo de lanzamiento", "Launch offer"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "classic",
        color: CORAL,
        tags: &["instagram", "promoción", "promo", "oferta", "sale", "joyas"],
        draw: launch_promo,
    },
    Design {
        folder: "social",
        file: "sticky-note",
        names: ("Nota adhesiva", "Sticky note"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "playful",
        color: MINT,
        tags: &["instagram", "nota", "note", "pregunta", "question"],
        draw: sticky_note,
    },
    Design {
        folder: "social",
        file: "small-habits",
        names: ("Pequeños hábitos", "Small habits"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "minimal",
        color: MINT,
        tags: &["instagram", "hábitos", "habits", "carrusel", "carousel"],
        draw: small_habits,
    },
    Design {
        folder: "social",
        file: "which-team",
        names: ("¿Qué team eres?", "Which team are you?"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &["instagram", "encuesta", "poll", "accesorios", "accessories"],
        draw: which_team,
    },
    Design {
        folder: "social",
        file: "skincare-routine",
        names: ("Rutina de skincare", "Skincare routine"),
        kind: "size.ig_portrait",
        page_format: "social",
        style: "minimal",
        color: MINT_DARK,
        tags: &["instagram", "skincare", "piel", "skin", "belleza", "beauty"],
        draw: skincare,
    },
    Design {
        folder: "invitations",
        file: "oval-party",
        names: ("Fiesta de cumpleaños en óvalo", "Oval birthday party"),
        kind: "size.invitation",
        page_format: "print",
        style: "playful",
        color: CORAL,
        tags: &[
            "cumpleaños",
            "birthday",
            "fiesta",
            "party",
            "lunares",
            "dots",
            "invitación",
            "invitation",
        ],
        draw: oval_party,
    },
    Design {
        folder: "invitations",
        file: "striped-party",
        names: ("Mi cumple a rayas", "Striped birthday"),
        kind: "size.invitation",
        page_format: "print",
        style: "playful",
        color: CORAL,
        tags: &[
            "cumpleaños",
            "birthday",
            "rayas",
            "stripes",
            "invitación",
            "invitation",
        ],
        draw: striped_party,
    },
    Design {
        folder: "invitations",
        file: "turning-21",
        names: ("Cumplo 21 años", "Turning 21"),
        kind: "size.invitation",
        page_format: "print",
        style: "classic",
        color: CORAL,
        tags: &["cumpleaños", "birthday", "21", "invitación", "invitation"],
        draw: twenty_first,
    },
    Design {
        folder: "invitations",
        file: "girls-night",
        names: ("Noche de amigas", "Girls' night"),
        kind: "size.invitation",
        page_format: "print",
        style: "classic",
        color: CORAL,
        tags: &[
            "amigas",
            "friends",
            "noche",
            "night",
            "fiesta",
            "invitación",
            "invitation",
        ],
        draw: girls_night,
    },
    Design {
        folder: "invitations",
        file: "julieta-25",
        names: ("Los 25 de Julieta", "Julieta turns 25"),
        kind: "size.invitation",
        page_format: "print",
        style: "minimal",
        color: MINT,
        tags: &[
            "cumpleaños",
            "birthday",
            "elegante",
            "elegant",
            "invitación",
            "invitation",
        ],
        draw: julieta_25,
    },
    Design {
        folder: "invitations",
        file: "star-party",
        names: ("Cumpleaños con estrellas", "Star birthday"),
        kind: "size.invitation",
        page_format: "print",
        style: "bold",
        color: CORAL,
        tags: &[
            "cumpleaños",
            "birthday",
            "estrellas",
            "stars",
            "invitación",
            "invitation",
        ],
        draw: star_party,
    },
    Design {
        folder: "invitations",
        file: "big-birthday",
        names: ("Cumple 25", "Big 25th"),
        kind: "size.invitation",
        page_format: "print",
        style: "bold",
        color: CORAL,
        tags: &[
            "cumpleaños",
            "birthday",
            "foto",
            "photo",
            "invitación",
            "invitation",
        ],
        draw: big_birthday,
    },
    Design {
        folder: "invitations",
        file: "thirty",
        names: ("Mis 30", "My 30th"),
        kind: "size.invitation",
        page_format: "print",
        style: "classic",
        color: MINT_DARK,
        tags: &["cumpleaños", "birthday", "30", "invitación", "invitation"],
        draw: thirty,
    },
    Design {
        folder: "invitations",
        file: "wedding",
        names: ("Nos casamos", "We're getting married"),
        kind: "size.invitation",
        page_format: "print",
        style: "classic",
        color: MINT,
        tags: &[
            "boda",
            "wedding",
            "novios",
            "matrimonio",
            "invitación",
            "invitation",
        ],
        draw: wedding,
    },
    Design {
        folder: "invitations",
        file: "baby-shower",
        names: ("Baby shower", "Baby shower"),
        kind: "size.invitation",
        page_format: "print",
        style: "playful",
        color: MINT,
        tags: &[
            "bebé",
            "baby",
            "shower",
            "nacimiento",
            "invitación",
            "invitation",
        ],
        draw: baby_shower,
    },
    Design {
        folder: "social",
        file: "tip-of-the-day",
        names: ("Consejo del día", "Tip of the day"),
        kind: "size.ig_square",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &["consejo", "tip", "educación", "instagram", "post"],
        draw: tip_of_the_day,
    },
    Design {
        folder: "social",
        file: "before-after",
        names: ("Antes y después", "Before and after"),
        kind: "size.ig_square",
        page_format: "social",
        style: "minimal",
        color: CORAL,
        tags: &[
            "antes",
            "before",
            "después",
            "after",
            "fotos",
            "photos",
            "instagram",
            "post",
        ],
        draw: before_after,
    },
    Design {
        folder: "social",
        file: "new-product",
        names: ("Nuevo producto", "New product"),
        kind: "size.ig_square",
        page_format: "social",
        style: "playful",
        color: CORAL,
        tags: &[
            "producto",
            "product",
            "tienda",
            "shop",
            "lanzamiento",
            "instagram",
            "post",
        ],
        draw: new_product,
    },
    Design {
        folder: "social",
        file: "bold-quote",
        names: ("Frase en grande", "Bold quote"),
        kind: "size.ig_square",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &[
            "frase",
            "quote",
            "motivación",
            "motivation",
            "instagram",
            "post",
        ],
        draw: bold_quote,
    },
    Design {
        folder: "social",
        file: "live-event",
        names: ("Charla en vivo", "Live talk"),
        kind: "size.ig_square",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &[
            "evento",
            "event",
            "en vivo",
            "live",
            "charla",
            "instagram",
            "post",
        ],
        draw: live_event,
    },
    Design {
        folder: "social",
        file: "recipe",
        names: ("Receta", "Recipe"),
        kind: "size.ig_square",
        page_format: "social",
        style: "warm",
        color: CORAL,
        tags: &[
            "receta",
            "recipe",
            "comida",
            "food",
            "cocina",
            "instagram",
            "post",
        ],
        draw: recipe,
    },
    Design {
        folder: "social",
        file: "flash-sale",
        names: ("Oferta relámpago", "Flash sale"),
        kind: "size.ig_square",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &[
            "oferta",
            "sale",
            "descuento",
            "discount",
            "instagram",
            "post",
        ],
        draw: flash_sale,
    },
    Design {
        folder: "social",
        file: "testimonial",
        names: ("Testimonio", "Testimonial"),
        kind: "size.ig_square",
        page_format: "social",
        style: "minimal",
        color: CORAL,
        tags: &[
            "testimonio",
            "review",
            "opinión",
            "cliente",
            "instagram",
            "post",
        ],
        draw: testimonial,
    },
    Design {
        folder: "social",
        file: "opening-hours",
        names: ("Horario", "Opening hours"),
        kind: "size.ig_square",
        page_format: "social",
        style: "classic",
        color: CORAL,
        tags: &["horario", "hours", "tienda", "negocio", "instagram", "post"],
        draw: opening_hours,
    },
    Design {
        folder: "social",
        file: "podcast",
        names: ("Episodio de podcast", "Podcast episode"),
        kind: "size.ig_square",
        page_format: "social",
        style: "bold",
        color: CORAL,
        tags: &[
            "podcast",
            "audio",
            "episodio",
            "episode",
            "instagram",
            "post",
        ],
        draw: podcast,
    },
    Design {
        folder: "print",
        file: "card-minimal-line",
        names: ("Tarjeta minimalista", "Minimal card"),
        kind: "size.business_card",
        page_format: "print",
        style: "minimal",
        color: CORAL,
        tags: &["tarjeta", "card", "contacto", "arquitectura"],
        draw: card_minimal_line,
    },
    Design {
        folder: "print",
        file: "card-bakery",
        names: ("Tarjeta de panadería", "Bakery card"),
        kind: "size.business_card",
        page_format: "print",
        style: "playful",
        color: CORAL,
        tags: &["tarjeta", "card", "negocio", "panadería", "bakery"],
        draw: card_split,
    },
    Design {
        folder: "print",
        file: "card-photographer",
        names: ("Tarjeta con foto", "Photo card"),
        kind: "size.business_card",
        page_format: "print",
        style: "minimal",
        color: CORAL,
        tags: &["tarjeta", "card", "foto", "photo", "fotógrafa"],
        draw: card_photo,
    },
    Design {
        folder: "print",
        file: "card-studio",
        names: ("Tarjeta de estudio", "Studio card"),
        kind: "size.business_card",
        page_format: "print",
        style: "bold",
        color: CORAL,
        tags: &["tarjeta", "card", "web", "estudio", "studio"],
        draw: card_bold,
    },
    Design {
        folder: "print",
        file: "diploma-classic",
        names: ("Diploma clásico", "Classic diploma"),
        kind: "size.a4_h",
        page_format: "print",
        style: "classic",
        color: CORAL,
        tags: &["diploma", "certificado", "certificate", "curso", "course"],
        draw: diploma_classic,
    },
    Design {
        folder: "print",
        file: "certificate-participation",
        names: (
            "Certificado de participación",
            "Certificate of participation",
        ),
        kind: "size.a4_h",
        page_format: "print",
        style: "minimal",
        color: CORAL,
        tags: &["certificado", "certificate", "congreso", "participación"],
        draw: certificate_modern,
    },
    Design {
        folder: "print",
        file: "kids-diploma",
        names: ("Diploma infantil", "Kids' diploma"),
        kind: "size.a4_h",
        page_format: "print",
        style: "playful",
        color: CORAL,
        tags: &["diploma", "niños", "kids", "escuela", "school", "lectura"],
        draw: kids_diploma,
    },
    Design {
        folder: "print",
        file: "award",
        names: ("Reconocimiento", "Award"),
        kind: "size.a4_h",
        page_format: "print",
        style: "classic",
        color: CORAL,
        tags: &[
            "reconocimiento",
            "award",
            "premio",
            "empleado",
            "certificado",
        ],
        draw: award_gold,
    },
    Design {
        folder: "documents",
        file: "resume-header",
        names: ("Currículum con encabezado", "Resume with header"),
        kind: "size.resume",
        page_format: "print",
        style: "bold",
        color: CORAL,
        tags: &["cv", "currículum", "resume", "foto", "photo"],
        draw: resume_header,
    },
    Design {
        folder: "documents",
        file: "resume-columns",
        names: ("Currículum en dos columnas", "Two-column resume"),
        kind: "size.resume",
        page_format: "print",
        style: "minimal",
        color: CORAL,
        tags: &["cv", "currículum", "resume", "desarrollador", "developer"],
        draw: resume_two_columns,
    },
    Design {
        folder: "documents",
        file: "resume-elegant",
        names: ("Currículum elegante", "Elegant resume"),
        kind: "size.resume",
        page_format: "print",
        style: "classic",
        color: CORAL,
        tags: &["cv", "currículum", "resume", "abogada", "lawyer"],
        draw: resume_elegant,
    },
    Design {
        folder: "print",
        file: "flyer-yoga",
        names: ("Volante de clases de yoga", "Yoga class flyer"),
        kind: "size.flyer_a4",
        page_format: "print",
        style: "minimal",
        color: CORAL,
        tags: &["volante", "flyer", "yoga", "clases", "bienestar"],
        draw: flyer_yoga,
    },
    Design {
        folder: "print",
        file: "flyer-real-estate",
        names: ("Volante inmobiliario", "Real estate flyer"),
        kind: "size.flyer_a4",
        page_format: "print",
        style: "classic",
        color: CORAL,
        tags: &["volante", "flyer", "casa", "house", "venta", "inmobiliaria"],
        draw: flyer_real_estate,
    },
    Design {
        folder: "print",
        file: "flyer-workshop",
        names: ("Volante de taller", "Workshop flyer"),
        kind: "size.flyer_a4",
        page_format: "print",
        style: "bold",
        color: CORAL,
        tags: &["volante", "flyer", "taller", "workshop", "cerámica"],
        draw: flyer_workshop,
    },
    Design {
        folder: "print",
        file: "flyer-restaurant",
        names: ("Volante de restaurante", "Restaurant flyer"),
        kind: "size.flyer_a4",
        page_format: "print",
        style: "classic",
        color: CORAL,
        tags: &["volante", "flyer", "menú", "menu", "restaurante", "comida"],
        draw: flyer_restaurant,
    },
];

// --------------------------------------------------------------- rendering

/// An empty store: the templates embed no asset, but the writer asks one.
#[derive(Default)]
struct MemoryStore(Mutex<BTreeMap<ContentHash, Vec<u8>>>);

impl ContentStore for MemoryStore {
    fn put(&self, src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        let mut bytes = Vec::new();
        src.read_to_end(&mut bytes).map_err(StoreError::Io)?;
        Err(StoreError::Io(io::Error::other(
            "the templates embed no asset",
        )))
    }

    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        let bytes = self.0.lock().expect("a lock").get(&hash).cloned();
        let bytes = bytes.ok_or(StoreError::NotFound { hash })?;
        Ok(Box::new(Cursor::new(bytes)))
    }

    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError> {
        Ok(self.0.lock().expect("a lock").contains_key(&hash))
    }
}

/// The renderer of the exports, with the fonts the app carries.
fn renderers() -> GraphPageRenderers {
    let fonts = gafira_theme::fonts()
        .iter()
        .fold(FontBook::new(), |book, font| {
            book.with(font.family, font.bytes)
        });
    let fonts = gafira_theme::library().iter().fold(fonts, |book, face| {
        book.with(
            &gafira_theme::face_key(face.family, face.bold, face.italic),
            face.bytes,
        )
    });
    GraphPageRenderers::new(
        Box::new(|| Box::new(CpuBackend::new())),
        Arc::new(gafira_platform::SystemClock::new()),
        Arc::new(fonts),
    )
}

/// The thumbnail of the first page, as a save makes it (spec 033).
fn thumbnail(document: &Document, pages: &GraphPageRenderers) -> Thumbnail {
    let page = document.pages().first().expect("a page");
    let px = page.size.to_px(document.dpi()).expect("a size");
    let scale = (f64::from(THUMBNAIL_SIDE) / px.width().max(px.height())).min(1.0);
    let side = |v: f64| (v * scale).round().clamp(1.0, f64::from(THUMBNAIL_SIDE)) as i32;
    let request = PageRenderRequest {
        page: page.id,
        time: Flicks::default(),
        scale: scale as f32,
        region: IRect::new(0, 0, side(px.width()), side(px.height())),
        transparent: false,
    };
    let image = pages
        .for_document(document, BTreeMap::new())
        .render(&request, &CancelToken::new())
        .expect("the page draws");
    Thumbnail {
        page: page.id,
        image,
    }
}

// ------------------------------------------------------------- palettes

/// The colours a design is drawn with, by the role each brand colour
/// plays in it (D-122 lets templates leave the brand colours): `ink` for
/// dark text and grounds, `accent` for calls and highlights, `second` for
/// a second colour, and their light and dark tones.
struct Palette {
    ink: &'static str,
    ink_light: &'static str,
    slate: &'static str,
    accent: &'static str,
    accent_light: &'static str,
    accent_soft: &'static str,
    second: &'static str,
    second_dark: &'static str,
    second_pale: &'static str,
    second_tint: &'static str,
    paper: &'static str,
    mist: &'static str,
    border: &'static str,
}

impl Palette {
    /// Each brand colour and the one it becomes.
    fn pairs(&self) -> [(&'static str, &'static str); 13] {
        [
            (INK, self.ink),
            (INK_LIGHT, self.ink_light),
            (SLATE, self.slate),
            (CORAL, self.accent),
            (CORAL_LIGHT, self.accent_light),
            ("#FFC9BF", self.accent_soft),
            (MINT, self.second),
            (MINT_DARK, self.second_dark),
            (MINT_PALE, self.second_pale),
            (MINT_TINT, self.second_tint),
            (CANVAS, self.paper),
            (MIST, self.mist),
            (BORDER, self.border),
        ]
    }
}

const COFFEE: Palette = Palette {
    ink: "#3B2A20",
    ink_light: "#5A4032",
    slate: "#6E5A4C",
    accent: "#C0703A",
    accent_light: "#D9935F",
    accent_soft: "#F0D2B8",
    second: "#A3B18A",
    second_dark: "#5B6B45",
    second_pale: "#EEF0E4",
    second_tint: "#E2C9A8",
    paper: "#F6EEE3",
    mist: "#EDE3D6",
    border: "#CDBBA7",
};
const BERRY: Palette = Palette {
    ink: "#4A1530",
    ink_light: "#6B2346",
    slate: "#7D4A60",
    accent: "#E0457B",
    accent_light: "#F06C9B",
    accent_soft: "#FFC2D6",
    second: "#F7B32B",
    second_dark: "#B5302F",
    second_pale: "#FFF3F7",
    second_tint: "#FFE0EA",
    paper: "#FFF8F5",
    mist: "#FBE4EC",
    border: "#E8B8C8",
};
const OCEAN: Palette = Palette {
    ink: "#0B2545",
    ink_light: "#13315C",
    slate: "#4A5D7A",
    accent: "#F4A259",
    accent_light: "#F7B97E",
    accent_soft: "#FBD9B8",
    second: "#2A9D8F",
    second_dark: "#1D6E64",
    second_pale: "#E8F4F8",
    second_tint: "#CFE8F0",
    paper: "#F7FAFC",
    mist: "#E1EAF2",
    border: "#B7C7D6",
};
const FOREST: Palette = Palette {
    ink: "#1F3B2D",
    ink_light: "#2E5240",
    slate: "#50645A",
    accent: "#E9C46A",
    accent_light: "#F0D58F",
    accent_soft: "#F6E7BC",
    second: "#6A994E",
    second_dark: "#386641",
    second_pale: "#F2F7EE",
    second_tint: "#DDEBD3",
    paper: "#FAFAF2",
    mist: "#E7EDE1",
    border: "#BFCDB8",
};
const VIOLET: Palette = Palette {
    ink: "#231942",
    ink_light: "#3D2C6B",
    slate: "#5E548E",
    accent: "#FF6F91",
    accent_light: "#FF8FAB",
    accent_soft: "#FFC6D3",
    second: "#9F86C0",
    second_dark: "#5E548E",
    second_pale: "#F4F0FA",
    second_tint: "#E0D6F2",
    paper: "#FBF9FE",
    mist: "#ECE6F5",
    border: "#C9BDE0",
};
const MONO: Palette = Palette {
    ink: "#1A1A1A",
    ink_light: "#333333",
    slate: "#555555",
    accent: "#B08D57",
    accent_light: "#C9A97A",
    accent_soft: "#EADBC2",
    second: "#8A9A8B",
    second_dark: "#4F5D50",
    second_pale: "#F4F4F1",
    second_tint: "#E5E5DF",
    paper: "#FAF8F4",
    mist: "#EDEBE6",
    border: "#CFCBC2",
};
const SUNSET: Palette = Palette {
    ink: "#2B193D",
    ink_light: "#3E2558",
    slate: "#5C4A6E",
    accent: "#FF5E5B",
    accent_light: "#FF8A5B",
    accent_soft: "#FFD4B8",
    second: "#FFC145",
    second_dark: "#E07A1F",
    second_pale: "#FFF6E5",
    second_tint: "#FFE8B8",
    paper: "#FFFBF5",
    mist: "#FBEBDD",
    border: "#F0C9A8",
};
const CRIMSON: Palette = Palette {
    ink: "#5C0A14",
    ink_light: "#7A1320",
    slate: "#7A4A50",
    accent: "#C8102E",
    accent_light: "#E04A5F",
    accent_soft: "#F7C6CD",
    second: "#F2C14E",
    second_dark: "#8C1C13",
    second_pale: "#FFF5F3",
    second_tint: "#FBE0E0",
    paper: "#FFF9F2",
    mist: "#F6E6E2",
    border: "#E3BFC0",
};

/// The palette of the design whose file is `file`; `None` keeps the brand
/// colours.
fn palette_of(file: &str) -> Option<&'static Palette> {
    Some(match file {
        "corner-coffee" => &COFFEE,
        "beauty-centre" | "plant-nursery" | "sticky-note" | "skincare-routine" | "thirty" => {
            &FOREST
        }
        "tips-carousel" | "live-workshop" | "creative-brief" | "final-project"
        | "marketing-agency" | "creative-routine" | "star-party" => &VIOLET,
        "summer-sale" | "tricks-thumbnail" | "jazz-night" | "marketing-plan"
        | "stationery-shop" | "month-in-numbers" | "big-birthday" => &SUNSET,
        "quote-of-the-day" | "business-card" | "certificate" | "resume" | "brand-manual"
        | "portfolio" | "interior-studio" | "launch-promo" | "wedding" => &MONO,
        "we-are-hiring" | "newsletter" | "store-banner" | "web-design" | "travel-agency"
        | "small-habits" | "julieta-25" | "baby-shower" => &OCEAN,
        "birthday-invitation"
        | "new-hobbies"
        | "happy-birthday"
        | "which-team"
        | "oval-party"
        | "girls-night" => &BERRY,
        "new-collection" | "striped-party" | "turning-21" => &CRIMSON,
        "tip-of-the-day" => &OCEAN,
        "before-after" => &MONO,
        "new-product" => &FOREST,
        "bold-quote" => &SUNSET,
        "live-event" => &VIOLET,
        "recipe" => &COFFEE,
        "flash-sale" => &BERRY,
        "testimonial" => &OCEAN,
        "opening-hours" => &CRIMSON,
        "podcast" => &SUNSET,
        "card-minimal-line" => &MONO,
        "card-bakery" => &SUNSET,
        "card-photographer" => &OCEAN,
        "card-studio" => &VIOLET,
        "diploma-classic" => &MONO,
        "certificate-participation" => &FOREST,
        "kids-diploma" => &SUNSET,
        "award" => &MONO,
        "resume-header" => &OCEAN,
        "resume-columns" => &FOREST,
        "resume-elegant" => &MONO,
        "flyer-yoga" => &FOREST,
        "flyer-real-estate" => &OCEAN,
        "flyer-workshop" => &SUNSET,
        "flyer-restaurant" => &CRIMSON,
        _ => return None,
    })
}

/// `#RRGGBB` of a colour object of the document.
fn hex_of(color: &Value) -> Option<String> {
    let c = |k: &str| color[k].as_f64().map(|v| (v * 255.0).round() as u8);
    Some(format!("#{:02X}{:02X}{:02X}", c("r")?, c("g")?, c("b")?))
}

/// Every colour of `value` that the palette names, swapped for its own.
fn repaint(value: &mut Value, palette: &Palette) {
    match value {
        Value::Object(map) => {
            if map.get("space").and_then(Value::as_str) == Some("srgb") {
                if let Some(hex) = hex_of(value)
                    && let Some((_, to)) = palette.pairs().iter().find(|(from, _)| *from == hex)
                {
                    let a = value["a"].as_f64().unwrap_or(1.0);
                    *value = rgba(to, a);
                }
                return;
            }
            for child in map.values_mut() {
                repaint(child, palette);
            }
        }
        Value::Array(list) => list.iter_mut().for_each(|v| repaint(v, palette)),
        _ => {}
    }
}

/// Stops when a shape lies over the middle of a photo frame: a click or a
/// dropped photo would reach the shape, not the frame.
fn covered_frames(page: &Value) {
    let nodes = page["nodes"].as_array().expect("nodes");
    let boxed = |n: &Value| {
        let at = &n["base"]["placement"];
        let get = |k: &str| at[k].as_f64().unwrap_or(0.0);
        (get("x"), get("y"), get("w"), get("h"))
    };
    for (i, frame) in nodes.iter().enumerate() {
        if frame["kind"]["type"] != "frame" {
            continue;
        }
        let (x, y, w, h) = boxed(frame);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        for above in &nodes[i + 1..] {
            if above["kind"]["type"] != "shape" {
                continue;
            }
            let (ax, ay, aw, ah) = boxed(above);
            let over = (ax..=ax + aw).contains(&cx) && (ay..=ay + ah).contains(&cy);
            assert!(
                !over,
                "a shape covers the photo frame at ({x:.0}, {y:.0}): {}",
                above["base"]["name"]
            );
        }
    }
}

/// The bleed of a printed template, in mm.
const BLEED_MM: f64 = 3.0;

/// `document` with the guides of its size in «Crear un diseño» on every
/// page (the folds, locked, and the safe margins) and, for print, a bleed
/// of `BLEED_MM` with every shape that touches an edge carried out to it.
fn finished(document: &Document, kind: &str, print: bool, palette: Option<&Palette>) -> Document {
    let mut value: Value =
        serde_json::from_str(&document.to_json().expect("json")).expect("a document");
    if let Some(palette) = palette {
        repaint(&mut value, palette);
    }
    let guides = gafira_ui::size_guides(kind).expect("a known size");
    let bleed = BLEED_MM * MM;
    for page in value["pages"].as_array_mut().expect("pages") {
        covered_frames(page);
        page["guides"] = serde_json::to_value(&guides).expect("guides");
        if !print {
            continue;
        }
        page["bleed"] = json!({ "value": BLEED_MM, "unit": "mm" });
        let k = match page["size"]["width"]["unit"].as_str() {
            Some("mm") => MM,
            Some("in") => INCH,
            _ => 1.0,
        };
        let w = page["size"]["width"]["value"].as_f64().expect("width") * k;
        let h = page["size"]["height"]["value"].as_f64().expect("height") * k;
        for node in page["nodes"].as_array_mut().expect("nodes") {
            let kind = node["kind"]["type"].as_str().unwrap_or_default();
            if kind != "shape" && kind != "frame" {
                continue;
            }
            let at = &mut node["base"]["placement"];
            let turned = at["rotation_deg"].as_f64().unwrap_or(0.0) != 0.0;
            if turned {
                continue;
            }
            let get = |at: &Value, k: &str| at[k].as_f64().unwrap_or(0.0);
            let (x, y, nw, nh) = (get(at, "x"), get(at, "y"), get(at, "w"), get(at, "h"));
            let (mut x0, mut y0, mut x1, mut y1) = (x, y, x + nw, y + nh);
            if x0 <= 0.5 {
                x0 -= bleed;
            }
            if y0 <= 0.5 {
                y0 -= bleed;
            }
            if x1 >= w - 0.5 {
                x1 += bleed;
            }
            if y1 >= h - 0.5 {
                y1 += bleed;
            }
            at["x"] = json!(x0);
            at["y"] = json!(y0);
            at["w"] = json!(x1 - x0);
            at["h"] = json!(y1 - y0);
        }
    }
    Document::from_json(value.to_string().as_bytes()).expect("a valid template")
}

fn main() {
    let root = Path::new("assets/templates");
    assert!(
        Path::new("assets/i18n").is_dir(),
        "run from the repository root"
    );
    let pages = renderers();
    let store = MemoryStore::default();
    for design in DESIGNS {
        for (lang, code, name, other) in [
            (Lang::Es, "es", design.names.0, design.names.1),
            (Lang::En, "en", design.names.1, design.names.0),
        ] {
            let document = finished(
                &(design.draw)(lang),
                design.kind,
                design.page_format == "print",
                palette_of(design.file),
            );
            // Its name in its language, and in the other one as words to
            // find it by: a search works in either language.
            let mut tags = vec![
                design.kind.to_owned(),
                format!("lang:{code}"),
                format!("title:{name}"),
                other.to_lowercase(),
            ];
            tags.extend(design.tags.iter().map(|t| (*t).to_owned()));
            let meta = TemplateMeta {
                tags,
                page_format: design.page_format.to_owned(),
                style: design.style.to_owned(),
                color: palette_of(design.file)
                    .map_or(design.color, |p| p.accent)
                    .to_owned(),
            };
            let thumb = thumbnail(&document, &pages);
            let dir = root.join(design.folder);
            std::fs::create_dir_all(&dir).expect("the folder");
            let path = dir.join(format!("{}.{code}.gaf", design.file));
            let mut bytes = Cursor::new(Vec::new());
            write_template(&mut bytes, &document, Some(&thumb), &meta, &store).expect("writes");
            std::fs::write(&path, bytes.into_inner()).expect("saves");
            println!("{}", path.display());
        }
    }
}
