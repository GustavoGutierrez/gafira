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
            d.text(
                p,
                (180.0, 260.0, 720.0),
                "“",
                font("DM Serif Display", 220.0, MINT).center(),
            );
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
        d.ellipse(p, (w * 0.07, 60.0, w * 0.2, w * 0.2), MINT, 1.0);
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
            let document = (design.draw)(lang);
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
                color: design.color.to_owned(),
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
