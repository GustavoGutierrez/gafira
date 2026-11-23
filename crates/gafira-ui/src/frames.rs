//! The frames of Elementos → Marcos in collections (spec 066, owner
//! request with references): basic shapes, film and photo, devices, torn
//! paper, flowers, blobs, letters and numbers, and the popular ones. Each
//! item is an outline the photo is cut by, or a device or film that holds
//! frames in its own drawing.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use gafira_color::{Color, Rgba};
use gafira_doc::{FrameNode, GroupNode, NodeKind, Placement, ShapeGeometry, ShapeNode, ShapePath};
use gafira_edit::NodeSpec;
use kurbo::{BezPath, Point, Shape as _};

use crate::canvas::Piece;
use crate::shapes::{finished, poly, star};

/// What a frame is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A shape of the Formas library.
    Piece(Piece),
    /// An outline made here.
    Made(Made),
    /// A letter or digit of a heavy font.
    Glyph(char),
    /// A device or a film that holds frames.
    Holder(Holder),
}

/// The outlines made here, beyond the library's shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Made {
    TriangleDown,
    Diamond,
    Star12,
    Seal24,
    ArrowUpDown,
    ParallelogramBack,
    Trapezoid,
    TrapezoidDown,
    Arch,
    UShape,
    Chamfer,
    Ticket,
    /// Torn paper, by its variant.
    Torn(u8),
    /// A flower of this many petals.
    Flower(u8),
    /// A blob, by its seed.
    Blob(u8),
    /// A circle with this many scallops round its edge.
    Scallop(u8),
    /// A flower of this many pointed petals.
    Pointed(u8),
    /// A daisy of this many narrow petals.
    Daisy(u8),
    /// A clover of this many round leaves.
    Clover(u8),
    /// A tulip.
    Tulip,
    /// A flower whose petals are of different sizes, by its seed.
    Bloom(u8),
}

/// The devices and films.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Holder {
    Phone,
    Tablet,
    Monitor,
    Laptop,
    Polaroid,
    WhiteBorder,
    FilmStrip,
    FilmSingle,
    /// A flower head on its stem with leaves, by its variant.
    Stem(u8),
    /// A photo in a border of small flowers, by its variant.
    FloralBorder(u8),
    /// A round photo in a coloured scalloped ring, by its variant.
    Ring(u8),
    /// A shape cut out of paper, a ragged paper edge round it.
    PaperCut(u8),
    /// A photo torn into pieces on a card.
    TornPieces(u8),
    /// A printed photo: black frame, cream border, taped, notebook card.
    Print(u8),
}

/// One frame of the panel: the words that find it (Spanish and English)
/// and what it is.
#[derive(Clone, Debug)]
pub(crate) struct Item {
    pub(crate) words: String,
    pub(crate) kind: Kind,
}

/// A collection: its name's catalogue key and its frames.
#[derive(Clone, Debug)]
pub(crate) struct Collection {
    pub(crate) key: &'static str,
    pub(crate) items: Vec<Item>,
}

fn item(_id: &str, words: &str, kind: Kind) -> Item {
    Item {
        words: words.to_owned(),
        kind,
    }
}

/// The collections, in the order of the panel; «Populares» last.
pub(crate) fn collections() -> &'static [Collection] {
    static ALL: OnceLock<Vec<Collection>> = OnceLock::new();
    ALL.get_or_init(|| {
        use Kind::{Glyph, Holder as H, Made as M, Piece as P};
        let basic = vec![
            item("square", "cuadrado square", P(Piece::Square)),
            item(
                "rounded",
                "redondeado rounded square",
                P(Piece::RoundedSquare),
            ),
            item("circle", "círculo circulo circle", P(Piece::Circle)),
            item(
                "triangle",
                "triángulo triangulo triangle",
                P(Piece::Triangle),
            ),
            item(
                "triangle-down",
                "triángulo invertido triangle down",
                M(Made::TriangleDown),
            ),
            item("diamond", "rombo diamante diamond", M(Made::Diamond)),
            item(
                "pentagon",
                "pentágono pentagono pentagon",
                P(Piece::Pentagon),
            ),
            item("hexagon", "hexágono hexagono hexagon", P(Piece::Hexagon)),
            item(
                "hexagon-flat",
                "hexágono plano hexagon flat",
                P(Piece::HexagonFlat),
            ),
            item("octagon", "octágono octagono octagon", P(Piece::Octagon)),
            item("decagon", "decágono decagono decagon", P(Piece::Decagon)),
            item("star-4", "estrella 4 star", P(Piece::Star4)),
            item("star-5", "estrella 5 star", P(Piece::Star5)),
            item("star-6", "estrella 6 star", P(Piece::Star6)),
            item("star-8", "estrella 8 star burst", P(Piece::Star8)),
            item("star-12", "estrella 12 star burst", M(Made::Star12)),
            item("seal", "sello insignia seal badge", P(Piece::Seal)),
            item(
                "seal-24",
                "sello dentado insignia seal badge",
                M(Made::Seal24),
            ),
            item(
                "arrow-right",
                "flecha derecha arrow right",
                P(Piece::ArrowRight),
            ),
            item(
                "arrow-left",
                "flecha izquierda arrow left",
                P(Piece::ArrowLeft),
            ),
            item("arrow-up", "flecha arriba arrow up", P(Piece::ArrowUp)),
            item("arrow-down", "flecha abajo arrow down", P(Piece::ArrowDown)),
            item(
                "arrow-left-right",
                "flecha doble arrow both",
                P(Piece::ArrowLeftRight),
            ),
            item(
                "arrow-up-down",
                "flecha vertical doble arrow up down",
                M(Made::ArrowUpDown),
            ),
            item(
                "hexagon-wide",
                "hexágono ancho banda band",
                P(Piece::Preparation),
            ),
            item(
                "parallelogram",
                "paralelogramo parallelogram",
                P(Piece::Data),
            ),
            item(
                "parallelogram-back",
                "paralelogramo inverso parallelogram",
                M(Made::ParallelogramBack),
            ),
            item("trapezoid", "trapecio trapezoid", M(Made::Trapezoid)),
            item(
                "trapezoid-down",
                "trapecio invertido trapezoid",
                M(Made::TrapezoidDown),
            ),
            item(
                "stadium",
                "píldora pildora pill stadium",
                P(Piece::Terminator),
            ),
            item(
                "speech-square",
                "bocadillo diálogo speech bubble",
                P(Piece::SpeechSquare),
            ),
            item(
                "speech-round",
                "bocadillo redondo speech bubble",
                P(Piece::SpeechRound),
            ),
            item("heart", "corazón corazon heart", P(Piece::Heart)),
            item("cross", "cruz cross plus", P(Piece::Cross)),
            item("cloud", "nube cloud", P(Piece::Cloud)),
            item("wave", "ola documento wave", P(Piece::Document)),
            item("bookmark", "marcador bookmark", P(Piece::Bookmark)),
            item("ribbon", "cinta banner ribbon", P(Piece::Ribbon)),
            item("chamfer", "esquinas cortadas chamfer", M(Made::Chamfer)),
            item("ticket", "ticket entrada", M(Made::Ticket)),
            item("arch", "arco ventana arch", M(Made::Arch)),
            item("u-shape", "u redondeada bottom round", M(Made::UShape)),
            item("drop", "gota drop", P(Piece::Drop)),
        ];
        let film = vec![
            item(
                "polaroid",
                "polaroid instantánea foto photo",
                H(Holder::Polaroid),
            ),
            item(
                "white-border",
                "borde blanco foto photo border",
                H(Holder::WhiteBorder),
            ),
            item(
                "film-strip",
                "película tira cine film strip",
                H(Holder::FilmStrip),
            ),
            item(
                "film-single",
                "película fotograma film frame",
                H(Holder::FilmSingle),
            ),
        ];
        let devices = vec![
            item(
                "phone",
                "teléfono móvil celular phone mobile",
                H(Holder::Phone),
            ),
            item(
                "monitor",
                "monitor pantalla ordenador computer screen",
                H(Holder::Monitor),
            ),
            item("tablet", "tableta tablet", H(Holder::Tablet)),
            item("laptop", "portátil portatil laptop", H(Holder::Laptop)),
        ];
        let flower = "flor pétalos flower petals";
        let mut flowers: Vec<Item> = [4_u8, 5, 6, 8, 10, 12]
            .into_iter()
            .map(|n| item(&format!("flower-{n}"), flower, M(Made::Flower(n))))
            .collect();
        flowers.extend(
            [10_u8, 14, 18].map(|n| item("", "flor festón scallop flower", M(Made::Scallop(n)))),
        );
        flowers.extend(
            [5_u8, 6, 8].map(|n| item("", "flor puntiaguda pointed flower", M(Made::Pointed(n)))),
        );
        flowers.extend(
            [12_u8, 16].map(|n| item("", "margarita daisy flor flower", M(Made::Daisy(n)))),
        );
        flowers
            .extend([3_u8, 4].map(|n| item("", "trébol trebol clover flor", M(Made::Clover(n)))));
        flowers.push(item(
            "",
            "tulipán tulipan tulip flor flower",
            M(Made::Tulip),
        ));
        flowers.extend((0..3).map(|s| item("", "flor bloom flower", M(Made::Bloom(s)))));
        flowers.extend((0..8).map(|v| {
            item(
                "",
                "flor tallo hojas flower stem leaves",
                H(Holder::Stem(v)),
            )
        }));
        flowers.extend((0..3).map(|v| {
            item(
                "",
                "borde flores marco floral border",
                H(Holder::FloralBorder(v)),
            )
        }));
        flowers.extend(
            (0..2).map(|v| item("", "anillo festón círculo ring flower", H(Holder::Ring(v)))),
        );
        let torn_words = "papel rasgado roto paper torn";
        let mut paper: Vec<Item> = (0..10)
            .map(|v| item(&format!("torn-{v}"), torn_words, M(Made::Torn(v))))
            .collect();
        paper.extend((0..4).map(|v| {
            item(
                "",
                "foto rota trozos papel torn pieces",
                H(Holder::TornPieces(v)),
            )
        }));
        paper.extend(
            (0..11).map(|v| item("", "recorte papel kraft cut paper", H(Holder::PaperCut(v)))),
        );
        paper.extend((0..4).map(|v| {
            item(
                "",
                "foto impresa cinta print tape paper",
                H(Holder::Print(v)),
            )
        }));
        let blobs = (0..6)
            .map(|s| {
                item(
                    &format!("blob-{s}"),
                    "mancha orgánica blob organic",
                    M(Made::Blob(s)),
                )
            })
            .collect();
        // The Spanish alphabet: Ñ after N (owner request).
        let letters = ('A'..='N')
            .chain(['Ñ'])
            .chain('O'..='Z')
            .map(|c| {
                item(
                    &format!("letter-{c}"),
                    &format!("letra letter {}", c.to_lowercase()),
                    Glyph(c),
                )
            })
            .collect();
        let numbers = ('0'..='9')
            .map(|c| {
                item(
                    &format!("number-{c}"),
                    &format!("número numero number {c}"),
                    Glyph(c),
                )
            })
            .collect();
        let popular = vec![
            item("p-square", "cuadrado square", P(Piece::Square)),
            item("p-rounded", "redondeado rounded", P(Piece::RoundedSquare)),
            item("p-circle", "círculo circle", P(Piece::Circle)),
            item("p-triangle", "triángulo triangle", P(Piece::Triangle)),
            item("p-diamond", "rombo diamond", M(Made::Diamond)),
            item("p-star", "estrella star", P(Piece::Star5)),
            item("p-seal", "sello seal", M(Made::Seal24)),
            item("p-arrow", "flecha arrow", P(Piece::ArrowRight)),
            item("p-heart", "corazón heart", P(Piece::Heart)),
            item("p-polaroid", "polaroid", H(Holder::Polaroid)),
            item("p-phone", "teléfono phone", H(Holder::Phone)),
            item("p-blob", "mancha blob", M(Made::Blob(0))),
        ];
        vec![
            Collection {
                key: "frames.basic",
                items: basic,
            },
            Collection {
                key: "frames.film",
                items: film,
            },
            Collection {
                key: "frames.devices",
                items: devices,
            },
            Collection {
                key: "frames.paper",
                items: paper,
            },
            Collection {
                key: "frames.flowers",
                items: flowers,
            },
            Collection {
                key: "frames.blobs",
                items: blobs,
            },
            Collection {
                key: "frames.letters",
                items: letters,
            },
            Collection {
                key: "frames.numbers",
                items: numbers,
            },
            Collection {
                key: "frames.popular",
                items: popular,
            },
        ]
    })
}

/// The frames whose words hold every word of `text`, each once.
pub(crate) fn search(text: &str) -> Vec<&'static Item> {
    let words: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut seen = std::collections::HashSet::new();
    collections()
        .iter()
        .flat_map(|c| &c.items)
        .filter(|item| {
            let own = item.words.to_lowercase();
            words.iter().all(|w| own.contains(w.as_str()))
        })
        .filter(|item| seen.insert(item.kind_key()))
        .collect()
}

impl Item {
    /// What makes two items the same frame (a popular one is also basic).
    fn kind_key(&self) -> String {
        format!("{:?}", self.kind)
    }
}

/// The outline of a single frame: its geometry, its width over height and
/// its corner radius as a share of its shorter side. `None` for a device
/// or film, which is a group.
pub(crate) fn outline(kind: Kind) -> Option<(ShapeGeometry, f64, f64)> {
    match kind {
        Kind::Piece(piece) => Some((
            piece.geometry(),
            piece.aspect(),
            if piece == Piece::RoundedSquare {
                1.0 / 8.0
            } else {
                0.0
            },
        )),
        Kind::Made(made) => {
            let (path, aspect) = finished(made_path(made));
            Some((ShapeGeometry::Path(path), aspect, 0.0))
        }
        Kind::Glyph(c) => {
            let (path, aspect) = glyph(c)?;
            Some((ShapeGeometry::Path(path), aspect, 0.0))
        }
        Kind::Holder(_) => None,
    }
}

/// The width over height of `kind`.
pub(crate) fn aspect(kind: Kind) -> f64 {
    match kind {
        Kind::Holder(holder) => holder_aspect(holder),
        other => outline(other).map_or(1.0, |(_, aspect, _)| aspect),
    }
}

/// The path of an outline made here, in any box (it is fitted after).
fn made_path(made: Made) -> BezPath {
    match made {
        Made::TriangleDown => poly(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]),
        Made::Diamond => poly(&[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)]),
        Made::Star12 => poly(&star(12, 0.72)),
        Made::Seal24 => poly(&star(24, 0.9)),
        Made::ArrowUpDown => poly(&[
            (0.5, 0.0),
            (1.0, 0.28),
            (0.7, 0.28),
            (0.7, 1.32),
            (1.0, 1.32),
            (0.5, 1.6),
            (0.0, 1.32),
            (0.3, 1.32),
            (0.3, 0.28),
            (0.0, 0.28),
        ]),
        Made::ParallelogramBack => poly(&[(0.0, 0.0), (1.35, 0.0), (1.7, 1.0), (0.35, 1.0)]),
        Made::Trapezoid => poly(&[(0.25, 0.0), (1.05, 0.0), (1.3, 1.0), (0.0, 1.0)]),
        Made::TrapezoidDown => poly(&[(0.0, 0.0), (1.3, 0.0), (1.05, 1.0), (0.25, 1.0)]),
        Made::Arch => {
            let mut path = BezPath::new();
            path.move_to((0.0, 1.4));
            path.line_to((0.0, 0.5));
            path.extend(
                kurbo::Arc::new(
                    (0.5, 0.5),
                    (0.5, 0.5),
                    std::f64::consts::PI,
                    std::f64::consts::PI,
                    0.0,
                )
                .append_iter(0.1),
            );
            path.line_to((1.0, 1.4));
            path.close_path();
            path
        }
        Made::UShape => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.0));
            path.line_to((1.0, 0.0));
            path.line_to((1.0, 0.9));
            path.extend(
                kurbo::Arc::new((0.5, 0.9), (0.5, 0.5), 0.0, std::f64::consts::PI, 0.0)
                    .append_iter(0.1),
            );
            path.close_path();
            path
        }
        Made::Chamfer => {
            let c = 0.16;
            poly(&[
                (c, 0.0),
                (1.0 - c, 0.0),
                (1.0, c),
                (1.0, 1.0 - c),
                (1.0 - c, 1.0),
                (c, 1.0),
                (0.0, 1.0 - c),
                (0.0, c),
            ])
        }
        Made::Ticket => ticket(),
        Made::Torn(variant) => torn(variant),
        Made::Scallop(n) => polar(|a| 0.9 + 0.1 * (f64::from(n) * a / 2.0).cos().abs()),
        Made::Pointed(n) => polar(|a| {
            let c = (f64::from(n) * a / 2.0).cos().abs();
            0.5 + 0.5 * c * c * c
        }),
        Made::Daisy(n) => polar(|a| 0.72 + 0.28 * (f64::from(n) * a / 2.0).cos().abs().powf(0.35)),
        Made::Clover(n) => polar(|a| 0.45 + 0.55 * (f64::from(n) * a / 2.0).cos().abs().powf(0.6)),
        Made::Tulip => tulip(),
        Made::Bloom(seed) => {
            let seed = u32::from(seed) + 31;
            polar(move |a| {
                let n = 7.0;
                let petal = (a / std::f64::consts::TAU * n).floor() as u32;
                let size = 0.82 + 0.18 * noise(seed, petal);
                size * (0.66 + 0.34 * (n * a / 2.0).cos().abs().sqrt())
            })
        }
        Made::Flower(petals) => flower(petals),
        Made::Blob(seed) => blob(seed),
    }
}

/// A closed outline of radius `r(angle)` round the centre, the angle from
/// straight up, clockwise.
fn polar(r: impl Fn(f64) -> f64) -> BezPath {
    let steps = 360;
    let points: Vec<(f64, f64)> = (0..steps)
        .map(|i| {
            let t = std::f64::consts::TAU * f64::from(i) / f64::from(steps);
            let a = t - std::f64::consts::FRAC_PI_2;
            let radius = r(t);
            (radius * a.cos(), radius * a.sin())
        })
        .collect();
    poly(&points)
}

/// A tulip: a round cup under three pointed petals.
fn tulip() -> BezPath {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.15));
    path.line_to((0.22, 0.45));
    path.line_to((0.5, 0.0));
    path.line_to((0.78, 0.45));
    path.line_to((1.0, 0.15));
    path.line_to((1.0, 0.6));
    path.curve_to((1.0, 0.85), (0.8, 1.0), (0.5, 1.0));
    path.curve_to((0.2, 1.0), (0.0, 0.85), (0.0, 0.6));
    path.close_path();
    path
}

/// A leaf: two arcs meeting at its ends, pointing right.
fn leaf() -> BezPath {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.5));
    path.quad_to((0.5, -0.25), (1.0, 0.5));
    path.quad_to((0.5, 1.25), (0.0, 0.5));
    path.close_path();
    path
}

/// `path` with its edge made ragged like torn paper: every point pushed
/// out or in from the middle by up to `amount` of its size.
fn ragged(path: &BezPath, amount: f64, seed: u32) -> BezPath {
    let b = path.bounding_box();
    let centre = b.center();
    let size = b.width().max(b.height());
    let mut points = Vec::new();
    kurbo::flatten(
        path.elements().iter().copied(),
        size / 160.0,
        |el| match el {
            kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => points.push(p),
            _ => {}
        },
    );
    let moved: Vec<(f64, f64)> = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let d = *p - centre;
            let len = d.hypot().max(1e-9);
            let push = (noise(seed, i as u32) - 0.35) * amount * size;
            let q = *p + d / len * push;
            (q.x, q.y)
        })
        .collect();
    poly(&moved)
}

/// A rectangle with its corners scooped out, like a ticket.
fn ticket() -> BezPath {
    let (w, h, r) = (1.3, 1.0, 0.14);
    let mut path = BezPath::new();
    path.move_to((r, 0.0));
    path.line_to((w - r, 0.0));
    path.quad_to((w - r, r), (w, r));
    path.line_to((w, h - r));
    path.quad_to((w - r, h - r), (w - r, h));
    path.line_to((r, h));
    path.quad_to((r, h - r), (0.0, h - r));
    path.line_to((0.0, r));
    path.quad_to((r, r), (r, 0.0));
    path.close_path();
    path
}

/// A number in `0..1` that `seed` and `i` always give.
fn noise(seed: u32, i: u32) -> f64 {
    let mut x = seed.wrapping_mul(0x9E37_79B9) ^ i.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    f64::from(x % 10_000) / 10_000.0
}

/// A sheet of torn paper: its long sides ragged (variant 0, 3 and 4 also
/// its short sides), 1.4 wide for 1 high.
fn torn(variant: u8) -> BezPath {
    if variant >= 5 {
        return torn_sides(variant);
    }
    let (w, h) = (1.4, 1.0);
    let seed = u32::from(variant) + 7;
    let rough =
        |i: u32, side: u32| (noise(seed + side, i) - 0.5) * if variant == 4 { 0.02 } else { 0.06 };
    let all = matches!(variant, 0 | 3 | 4);
    let top = variant != 2;
    let bottom = variant != 1;
    let steps = 28;
    let mut points = Vec::new();
    for i in 0..=steps {
        let x = w * f64::from(i) / f64::from(steps);
        points.push((x, if top { 0.04 + rough(i, 0) } else { 0.0 }));
    }
    for i in 1..steps / 2 {
        let y = h * f64::from(i) / f64::from(steps / 2);
        points.push((if all { w - 0.03 + rough(i, 1) } else { w }, y));
    }
    for i in (0..=steps).rev() {
        let x = w * f64::from(i) / f64::from(steps);
        points.push((x, if bottom { h - 0.04 + rough(i, 2) } else { h }));
    }
    for i in (1..steps / 2).rev() {
        let y = h * f64::from(i) / f64::from(steps / 2);
        points.push((if all { 0.03 + rough(i, 3) } else { 0.0 }, y));
    }
    poly(&points)
}

/// More torn sheets: one side (left, right), two sides, a strip, a tall
/// sheet.
fn torn_sides(variant: u8) -> BezPath {
    // Which sides are torn: top, right, bottom, left; and the size.
    let (sides, w, h) = match variant {
        5 => ([false, false, false, true], 1.3, 1.0),
        6 => ([false, true, false, false], 1.3, 1.0),
        7 => ([true, true, false, false], 1.2, 1.0),
        8 => ([true, false, true, false], 3.0, 1.0),
        _ => ([true, false, true, true], 0.8, 1.0),
    };
    let seed = u32::from(variant) + 51;
    let rough = |i: u32, side: u32| (noise(seed + side, i) - 0.5) * 0.06;
    let edge = |torn: bool, i: u32, side: u32| if torn { 0.04 + rough(i, side) } else { 0.0 };
    let steps = 24_u32;
    let mut points = Vec::new();
    for i in 0..=steps {
        let t = f64::from(i) / f64::from(steps);
        points.push((w * t, edge(sides[0], i, 0)));
    }
    for i in 1..steps {
        let t = f64::from(i) / f64::from(steps);
        points.push((w - edge(sides[1], i, 1), h * t));
    }
    for i in (0..=steps).rev() {
        let t = f64::from(i) / f64::from(steps);
        points.push((w * t, h - edge(sides[2], i, 2)));
    }
    for i in (1..steps).rev() {
        let t = f64::from(i) / f64::from(steps);
        points.push((edge(sides[3], i, 3), h * t));
    }
    poly(&points)
}

/// A flower of `petals` round petals.
fn flower(petals: u8) -> BezPath {
    let n = f64::from(petals);
    let steps = 240;
    let points: Vec<(f64, f64)> = (0..steps)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / f64::from(steps)
                - std::f64::consts::FRAC_PI_2;
            let r = 0.62 + 0.38 * (n * a / 2.0).cos().abs().sqrt();
            (r * a.cos(), r * a.sin())
        })
        .collect();
    poly(&points)
}

/// A smooth blob through seven points at radii `seed` chooses.
fn blob(seed: u8) -> BezPath {
    let n = 7;
    let seed = u32::from(seed) + 101;
    let points: Vec<Point> = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            let r = 0.7 + 0.3 * noise(seed, i);
            Point::new(r * a.cos(), r * a.sin())
        })
        .collect();
    // A closed Catmull-Rom curve as cubic Béziers.
    let at = |i: i32| points[i.rem_euclid(n as i32) as usize];
    let mut path = BezPath::new();
    path.move_to(at(0));
    for i in 0..n as i32 {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = p1 + (p2 - p0) / 6.0;
        let c2 = p2 - (p3 - p1) / 6.0;
        path.curve_to(c1, c2, p2);
    }
    path.close_path();
    path
}

/// The glyph outlines drawn so far.
type Glyphs = Mutex<HashMap<char, Option<(ShapePath, f64)>>>;

/// The outline of the letter or digit `c` in Archivo Black, fitted to the
/// unit box; kept once drawn.
fn glyph(c: char) -> Option<(ShapePath, f64)> {
    static MADE: OnceLock<Glyphs> = OnceLock::new();
    let made = MADE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(found) = made.lock().ok()?.get(&c) {
        return found.clone();
    }
    let style = gafira_doc::SpanStyle {
        font: "Archivo Black".to_owned(),
        size: 400.0,
        color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
        bold: false,
        italic: false,
    };
    let text = gafira_doc::TextNode::plain(&c.to_string(), &style);
    let outline = gafira_render::text_outline(&text, (1000.0, 1000.0), crate::canvas::text_fonts())
        .map(|(path, _, _)| path)
        .filter(|path| path.bounding_box().area() > 1.0)
        .map(finished);
    if let Ok(mut made) = made.lock() {
        made.insert(c, outline.clone());
    }
    outline
}

fn holder_aspect(holder: Holder) -> f64 {
    match holder {
        Holder::Phone => 0.5,
        Holder::Tablet => 0.75,
        Holder::Monitor => 1.25,
        Holder::Laptop => 1.6,
        Holder::Polaroid => 0.85,
        Holder::WhiteBorder => 1.0,
        Holder::FilmStrip => 2.6,
        Holder::FilmSingle => 1.25,
        Holder::Stem(_) => 0.62,
        Holder::FloralBorder(v) => {
            if v == 1 {
                0.8
            } else {
                1.35
            }
        }
        Holder::Ring(_) => 1.0,
        Holder::PaperCut(v) => {
            let (path, _) = paper_cut(v);
            let b = path.bounding_box();
            b.width() / b.height()
        }
        Holder::TornPieces(v) => {
            if v == 3 {
                1.3
            } else {
                0.85
            }
        }
        Holder::Print(v) => {
            if v == 3 {
                0.82
            } else {
                0.85
            }
        }
    }
}

/// The shapes a paper cut-out is cut to, and its paper: white, kraft,
/// dark kraft.
fn paper_cut(variant: u8) -> (BezPath, u32) {
    use kurbo::Shape as _;
    let white = 0xFBFBF8;
    let kraft = 0xC8A27A;
    let dark = 0x8D6E4E;
    let ellipse =
        |w: f64, h: f64| kurbo::Ellipse::from_rect(kurbo::Rect::new(0.0, 0.0, w, h)).to_path(0.01);
    match variant {
        0 => (ellipse(0.75, 1.0), white),
        1 => (made_path(Made::Arch), white),
        2 => (ellipse(1.0, 1.0), white),
        3 => (ellipse(1.4, 1.0), white),
        4 => (crate::shapes::piece_path(Piece::Heart), white),
        5 => (crate::shapes::piece_path(Piece::Heart), kraft),
        6 => (crate::shapes::piece_path(Piece::Star5), white),
        7 => (made_path(Made::Flower(8)), white),
        8 => (kurbo::Rect::new(0.0, 0.0, 1.0, 1.0).to_path(0.01), kraft),
        9 => (kurbo::Rect::new(0.0, 0.0, 1.0, 1.0).to_path(0.01), white),
        _ => (kurbo::Rect::new(0.0, 0.0, 1.2, 1.0).to_path(0.01), dark),
    }
}

const DARK: u32 = 0x1C1C1E;
const GREY: u32 = 0x9AA0A6;
const WHITE: u32 = 0xFFFFFF;

/// A shape of `path` filled with `colour` in the box `(x, y, w, h)`,
/// turned `degrees`.
fn drawn(path: BezPath, colour: u32, (x, y, w, h): (f64, f64, f64, f64), degrees: f64) -> NodeSpec {
    let (path, _) = finished(path);
    let mut shape = ShapeNode::new(ShapeGeometry::Path(path));
    shape.fill = Some(crate::colors::hex(colour));
    let mut placement = Placement::new(x, y, w, h);
    placement.rotation_deg = degrees;
    NodeSpec::new(NodeKind::Shape(shape), placement)
}

/// An empty frame cut by `path`, in the box `(x, y, w, h)`.
fn framed(path: BezPath, (x, y, w, h): (f64, f64, f64, f64)) -> NodeSpec {
    let (path, _) = finished(path);
    NodeSpec::new(
        NodeKind::Frame(FrameNode::new(ShapeGeometry::Path(path))),
        Placement::new(x, y, w, h),
    )
}

/// A filled rectangle of `colour`, its corners rounded by `radius` px.
fn body(colour: u32, (x, y, w, h): (f64, f64, f64, f64), radius: f64) -> NodeSpec {
    let mut shape = ShapeNode::new(ShapeGeometry::Rect);
    shape.fill = Some(crate::colors::hex(colour));
    shape.corner_radius = radius;
    NodeSpec::new(NodeKind::Shape(shape), Placement::new(x, y, w, h))
}

/// An empty rectangular frame, its corners rounded by `radius` px.
fn screen((x, y, w, h): (f64, f64, f64, f64), radius: f64) -> NodeSpec {
    let mut frame = FrameNode::new(ShapeGeometry::Rect);
    frame.corner_radius = radius;
    NodeSpec::new(NodeKind::Frame(frame), Placement::new(x, y, w, h))
}

/// A device or film in the box `(x, y, w, h)`: its drawing and the frames
/// it holds, as one group.
pub(crate) fn holder(holder: Holder, (x, y, w, h): (f64, f64, f64, f64)) -> NodeSpec {
    let mut parts = Vec::new();
    match holder {
        Holder::Phone | Holder::Tablet => {
            let (round, inset) = if holder == Holder::Phone {
                (0.14, 0.045)
            } else {
                (0.06, 0.05)
            };
            parts.push(body(DARK, (x, y, w, h), w * round));
            let i = w * inset;
            parts.push(screen(
                (x + i, y + i, w - 2.0 * i, h - 2.0 * i),
                w * (round - inset).max(0.01),
            ));
            if holder == Holder::Phone {
                parts.push(body(
                    DARK,
                    (x + w * 0.35, y + i, w * 0.3, h * 0.025),
                    h * 0.0125,
                ));
            }
        }
        Holder::Monitor => {
            let top = h * 0.78;
            parts.push(body(DARK, (x, y, w, top), w * 0.02));
            let i = w * 0.03;
            parts.push(screen((x + i, y + i, w - 2.0 * i, top - 2.0 * i), 0.0));
            parts.push(body(GREY, (x + w * 0.44, y + top, w * 0.12, h * 0.15), 0.0));
            parts.push(body(
                GREY,
                (x + w * 0.3, y + h * 0.93, w * 0.4, h * 0.05),
                h * 0.02,
            ));
        }
        Holder::Laptop => {
            let lid = h * 0.86;
            parts.push(body(DARK, (x + w * 0.1, y, w * 0.8, lid), w * 0.02));
            let i = w * 0.025;
            parts.push(screen(
                (x + w * 0.1 + i, y + i, w * 0.8 - 2.0 * i, lid - 2.0 * i),
                0.0,
            ));
            parts.push(body(GREY, (x, y + lid, w, h - lid), (h - lid) * 0.5));
        }
        Holder::Polaroid | Holder::WhiteBorder => {
            let mut sheet = ShapeNode::new(ShapeGeometry::Rect);
            sheet.fill = Some(crate::colors::hex(WHITE));
            sheet.stroke = Some(gafira_doc::ShapeStroke::new(
                crate::colors::hex(0xD9D9D9),
                (w * 0.006).max(1.0),
            ));
            parts.push(NodeSpec::new(
                NodeKind::Shape(sheet),
                Placement::new(x, y, w, h),
            ));
            let i = w * 0.06;
            let bottom = if holder == Holder::Polaroid {
                h * 0.22
            } else {
                i
            };
            parts.push(screen((x + i, y + i, w - 2.0 * i, h - i - bottom), 0.0));
        }
        Holder::FilmStrip | Holder::FilmSingle => {
            parts.push(body(DARK, (x, y, w, h), 0.0));
            let band = h * 0.16;
            // The holes along both edges.
            let hole = band * 0.45;
            let holes = ((w / (hole * 2.2)).floor() as usize).max(2);
            let step = w / holes as f64;
            for k in 0..holes {
                let hx = x + step * (k as f64 + 0.5) - hole / 2.0;
                for hy in [y + (band - hole) / 2.0, y + h - band + (band - hole) / 2.0] {
                    parts.push(body(WHITE, (hx, hy, hole, hole), hole * 0.2));
                }
            }
            let count = if holder == Holder::FilmStrip { 3 } else { 1 };
            let gap = w * 0.02;
            let fw = (w - gap * (count as f64 + 1.0)) / count as f64;
            for k in 0..count {
                let fx = x + gap + (fw + gap) * k as f64;
                parts.push(screen((fx, y + band, fw, h - 2.0 * band), 0.0));
            }
        }
        Holder::Stem(_)
        | Holder::FloralBorder(_)
        | Holder::Ring(_)
        | Holder::PaperCut(_)
        | Holder::TornPieces(_)
        | Holder::Print(_) => {}
    }
    if let Some(more) = held(holder, (x, y, w, h)) {
        parts.extend(more);
    }
    NodeSpec::new(NodeKind::Group(GroupNode {}), Placement::new(x, y, w, h)).with_children(parts)
}

/// The parts of the flower and paper holders in the box `(x, y, w, h)`.
fn held(holder: Holder, (x, y, w, h): (f64, f64, f64, f64)) -> Option<Vec<NodeSpec>> {
    let mut parts = Vec::new();
    match holder {
        Holder::Stem(v) => {
            // The head, its stem, and leaves in the stem's colour.
            let (head, colour, leaves) = [
                (Made::Flower(5), 0x8BC34A, 0),
                (Made::Tulip, 0x1E9BD7, 1),
                (Made::Clover(4), 0x1B5E20, 2),
                (Made::Pointed(6), 0x1E9BD7, 0),
                (Made::Daisy(14), 0x5C5AA8, 3),
                (Made::Flower(8), 0xC0D72F, 0),
                (Made::Bloom(1), 0xF4A62A, 3),
                (Made::Scallop(12), 0x1B5E20, 2),
            ][usize::from(v % 8)];
            let head_h = w * 0.95;
            let stem_w = w * 0.07;
            let top = y + head_h * 0.85;
            parts.push(body(
                colour,
                (x + (w - stem_w) / 2.0, top, stem_w, y + h - top),
                0.0,
            ));
            let mid = y + head_h + (h - head_h) * 0.45;
            match leaves {
                0 => {
                    parts.push(drawn(
                        leaf(),
                        colour,
                        (x + w * 0.12, mid, w * 0.38, w * 0.16),
                        200.0,
                    ));
                    parts.push(drawn(
                        leaf(),
                        colour,
                        (x + w * 0.5, mid, w * 0.38, w * 0.16),
                        -20.0,
                    ));
                }
                1 => {
                    let bow = poly(&[(0.0, 0.0), (0.5, 0.5), (0.0, 1.0)]);
                    parts.push(drawn(
                        bow,
                        colour,
                        (x + w * 0.22, mid, w * 0.28, w * 0.28),
                        0.0,
                    ));
                    let bow = poly(&[(1.0, 0.0), (0.5, 0.5), (1.0, 1.0)]);
                    parts.push(drawn(
                        bow,
                        colour,
                        (x + w * 0.5, mid, w * 0.28, w * 0.28),
                        0.0,
                    ));
                }
                2 => {
                    let heart = crate::shapes::piece_path(Piece::Heart);
                    parts.push(drawn(
                        heart,
                        colour,
                        (x + w * 0.3, y + h - w * 0.4, w * 0.4, w * 0.36),
                        0.0,
                    ));
                }
                _ => {
                    parts.push(drawn(
                        leaf(),
                        colour,
                        (x + w * 0.5, mid, w * 0.45, w * 0.2),
                        -35.0,
                    ));
                }
            }
            parts.push(framed(made_path(head), (x, y, w, head_h)));
        }
        Holder::FloralBorder(v) => {
            // A card, the photo inside, and small flowers all round it.
            let ground = [0xFFFFFF, 0xFFE27A, 0xF39C34][usize::from(v % 3)];
            if v > 0 {
                parts.push(body(ground, (x, y, w, h), 0.0));
            }
            let band = w.min(h) * 0.12;
            parts.push(screen(
                (x + band, y + band, w - 2.0 * band, h - 2.0 * band),
                0.0,
            ));
            let colours = [0xF06292, 0x64B5F6, 0xBA68C8, 0xFFB74D, 0xE57373, 0x81C784];
            let size = band * 0.85;
            let along = 2.0 * (w + h - 2.0 * band);
            let count = (along / (size * 1.25)).floor().max(8.0) as u32;
            for k in 0..count {
                let t = f64::from(k) / f64::from(count) * along;
                let (inner_w, inner_h) = (w - band, h - band);
                let (cx, cy) = if t < inner_w {
                    (x + band / 2.0 + t, y + band / 2.0)
                } else if t < inner_w + inner_h {
                    (x + w - band / 2.0, y + band / 2.0 + t - inner_w)
                } else if t < 2.0 * inner_w + inner_h {
                    (
                        x + w - band / 2.0 - (t - inner_w - inner_h),
                        y + h - band / 2.0,
                    )
                } else {
                    (
                        x + band / 2.0,
                        y + h - band / 2.0 - (t - 2.0 * inner_w - inner_h),
                    )
                };
                let jitter = (noise(u32::from(v) + 3, k) - 0.5) * band * 0.3;
                let s = size * (0.75 + 0.35 * noise(u32::from(v) + 9, k));
                let colour = colours[(k as usize + usize::from(v)) % colours.len()];
                parts.push(drawn(
                    made_path(Made::Flower(5)),
                    colour,
                    (cx - s / 2.0 + jitter, cy - s / 2.0 - jitter, s, s),
                    f64::from(k) * 37.0,
                ));
            }
        }
        Holder::Ring(v) => {
            let colour = [0x29B6F6, 0xA5D6E7][usize::from(v % 2)];
            parts.push(drawn(
                made_path(Made::Scallop(12)),
                colour,
                (x, y, w, h),
                0.0,
            ));
            let i = w * 0.1;
            let ring =
                kurbo::Ellipse::from_rect(kurbo::Rect::new(0.0, 0.0, 1.0, 1.0)).to_path(0.01);
            parts.push(framed(ring, (x + i, y + i, w - 2.0 * i, h - 2.0 * i)));
        }
        Holder::PaperCut(v) => {
            // The paper: the shape a little larger, its edge ragged; the
            // photo cut by the shape inside.
            let (shape, paper) = paper_cut(v);
            let edge = ragged(&shape, 0.035, u32::from(v) + 17);
            let mut paper_spec = drawn(edge, paper, (x, y, w, h), 0.0);
            if let NodeKind::Shape(node) = &mut paper_spec.kind {
                node.stroke = Some(gafira_doc::ShapeStroke::new(
                    crate::colors::hex(0xD5D0C8),
                    (w * 0.004).max(0.5),
                ));
            }
            parts.push(paper_spec);
            let i = w.min(h) * 0.09;
            parts.push(framed(shape, (x + i, y + i, w - 2.0 * i, h - 2.0 * i)));
        }
        Holder::TornPieces(v) => {
            // A light card, and the photo in two or three pieces with torn
            // edges between them.
            parts.push(body(0xF1F1F3, (x, y, w, h), w.min(h) * 0.06));
            let i = w.min(h) * 0.08;
            let (ix, iy, iw, ih) = (x + i, y + i, w - 2.0 * i, h - 2.0 * i);
            let gap = iw.min(ih) * 0.03;
            let seed = u32::from(v) + 61;
            // A tear from top to bottom at `at` of the width (or from left
            // to right for variant 3), its points.
            let tear = |at: f64, n: u32| -> Vec<f64> {
                (0..=12)
                    .map(|k| at + (noise(seed + n, k) - 0.5) * 0.08)
                    .collect()
            };
            let cuts: Vec<Vec<f64>> = match v {
                0 => vec![tear(0.5, 0)],
                1 => vec![
                    (0..=12)
                        .map(|k| 0.3 + 0.4 * f64::from(k) / 12.0 + (noise(seed, k) - 0.5) * 0.06)
                        .collect(),
                ],
                2 => vec![tear(0.33, 0), tear(0.66, 1)],
                _ => vec![tear(0.5, 0)],
            };
            let across = v == 3;
            let mut edges: Vec<Vec<f64>> = vec![vec![0.0; 13]];
            edges.extend(cuts);
            edges.push(vec![1.0; 13]);
            for pair in edges.windows(2) {
                let (left, right) = (&pair[0], &pair[1]);
                let mut points = Vec::new();
                let along = |k: usize| k as f64 / 12.0;
                let half = gap / 2.0;
                for (k, t) in left.iter().enumerate() {
                    let shift = if *t > 0.0 { half } else { 0.0 };
                    points.push(if across {
                        (iw * along(k), ih * t + shift)
                    } else {
                        (iw * t + shift, ih * along(k))
                    });
                }
                for (k, t) in right.iter().enumerate().rev() {
                    let shift = if *t < 1.0 { half } else { 0.0 };
                    points.push(if across {
                        (iw * along(k), ih * t - shift)
                    } else {
                        (iw * t - shift, ih * along(k))
                    });
                }
                let piece = poly(&points);
                let b = piece.bounding_box();
                parts.push(framed(piece, (ix + b.x0, iy + b.y0, b.width(), b.height())));
            }
        }
        Holder::Print(v) => match v {
            0 => {
                parts.push(body(0x222222, (x, y, w, h), 0.0));
                let i = w * 0.07;
                parts.push(screen((x + i, y + i, w - 2.0 * i, h - 2.0 * i), 0.0));
            }
            1 => {
                parts.push(body(0xF3EBDD, (x, y, w, h), 0.0));
                let i = w * 0.09;
                parts.push(screen((x + i, y + i, w - 2.0 * i, h - 2.0 * i), 0.0));
            }
            2 => {
                // A kraft card with a row of punched holes under the photo.
                parts.push(body(0xB88A5E, (x, y, w, h), 0.0));
                let i = w * 0.08;
                parts.push(screen((x + i, y + i, w - 2.0 * i, h * 0.78), 0.0));
                let holes = 9;
                for k in 0..holes {
                    let d = w * 0.035;
                    let hx = x + i + (w - 2.0 * i - d) * f64::from(k) / f64::from(holes - 1);
                    parts.push(body(0xFFFFFF, (hx, y + h * 0.92, d, d), d / 2.0));
                }
            }
            _ => {
                // A white print, turned a little, held by a piece of tape.
                let mut print = body(
                    0xFFFFFF,
                    (x + w * 0.04, y + h * 0.06, w * 0.92, h * 0.9),
                    0.0,
                );
                print.placement.rotation_deg = -3.0;
                if let NodeKind::Shape(node) = &mut print.kind {
                    node.stroke = Some(gafira_doc::ShapeStroke::new(
                        crate::colors::hex(0xDDDDDD),
                        (w * 0.004).max(0.5),
                    ));
                }
                parts.push(print);
                let i = w * 0.09;
                let mut photo = screen(
                    (x + i, y + h * 0.06 + i, w - 2.0 * i, h * 0.9 - 2.0 * i),
                    0.0,
                );
                photo.placement.rotation_deg = -3.0;
                parts.push(photo);
                let mut tape = body(0xEADBB8, (x + w * 0.32, y, w * 0.36, h * 0.1), 0.0);
                tape.placement.rotation_deg = 6.0;
                parts.push(tape);
            }
        },
        _ => return None,
    }
    Some(parts)
}

/// `kind` as what goes on a page of `page_px`: half the page's shorter side
/// (three fifths for a device or film), centred on `centre` (page px).
pub(crate) fn spec(kind: Kind, (pw, ph): (f64, f64), centre: (f64, f64)) -> Option<NodeSpec> {
    let ratio = aspect(kind);
    let side = pw.min(ph)
        * if matches!(kind, Kind::Holder(_)) {
            0.6
        } else {
            0.5
        };
    let (w, h) = if ratio >= 1.0 {
        (side, side / ratio)
    } else {
        (side * ratio, side)
    };
    let (x, y) = (centre.0 - w / 2.0, centre.1 - h / 2.0);
    match kind {
        Kind::Holder(h_kind) => Some(holder(h_kind, (x, y, w, h))),
        other => {
            let (geometry, _, corner) = outline(other)?;
            let mut frame = FrameNode::new(geometry);
            frame.corner_radius = corner * w.min(h);
            Some(NodeSpec::new(
                NodeKind::Frame(frame),
                Placement::new(x, y, w, h),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Owner request: the collections of the reference, «Formas básicas»
    // with every basic shape; every frame has an outline or a holder.
    #[test]
    fn every_frame_has_its_outline() {
        let all = collections();
        let keys: Vec<&str> = all.iter().map(|c| c.key).collect();
        assert_eq!(
            keys,
            [
                "frames.basic",
                "frames.film",
                "frames.devices",
                "frames.paper",
                "frames.flowers",
                "frames.blobs",
                "frames.letters",
                "frames.numbers",
                "frames.popular"
            ]
        );
        assert!(all[0].items.len() >= 40, "the basic shapes");
        assert!(all[3].items.len() >= 29, "paper");
        assert!(all[4].items.len() >= 33, "flowers");
        assert_eq!(all[6].items.len(), 27, "A to Z and Ñ");
        assert_eq!(all[7].items.len(), 10);
        for item in all.iter().flat_map(|c| &c.items) {
            match item.kind {
                Kind::Holder(_) => {}
                kind => {
                    let (_, aspect, _) = outline(kind).unwrap_or_else(|| panic!("{}", item.words));
                    assert!(
                        aspect.is_finite() && aspect > 0.1 && aspect < 10.0,
                        "{}",
                        item.words
                    );
                }
            }
        }
    }

    // A device holds its screen as a frame inside its own drawing.
    #[test]
    fn a_phone_holds_a_screen() {
        let spec = spec(
            Kind::Holder(Holder::Phone),
            (1000.0, 1000.0),
            (500.0, 500.0),
        )
        .expect("a spec");
        assert!(matches!(spec.kind, NodeKind::Group(_)));
        let frames = spec
            .children
            .iter()
            .filter(|c| matches!(c.kind, NodeKind::Frame(_)))
            .count();
        assert_eq!(frames, 1);
        let screen = spec
            .children
            .iter()
            .find(|c| matches!(c.kind, NodeKind::Frame(_)))
            .expect("screen");
        assert!(screen.placement.w < spec.placement.w && screen.placement.h < spec.placement.h);
        let strip = holder(Holder::FilmStrip, (0.0, 0.0, 260.0, 100.0));
        assert_eq!(
            strip
                .children
                .iter()
                .filter(|c| matches!(c.kind, NodeKind::Frame(_)))
                .count(),
            3
        );
    }

    // The search finds frames by their words in Spanish or English.
    #[test]
    fn frames_are_found_by_their_words() {
        let heart = Kind::Piece(Piece::Heart);
        assert!(search("corazón").iter().any(|i| i.kind == heart));
        assert!(search("heart").iter().any(|i| i.kind == heart));
        assert!(search("letra b").iter().any(|i| i.kind == Kind::Glyph('B')));
        assert!(search("letra ñ").iter().any(|i| i.kind == Kind::Glyph('Ñ')));
        assert!(
            search("phone")
                .iter()
                .any(|i| i.kind == Kind::Holder(Holder::Phone))
        );
        assert!(search("zzz").is_empty());
        let stars = search("estrella");
        assert!(stars.len() >= 5);
    }
}
