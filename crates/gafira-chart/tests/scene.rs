//! S83: what draws a chart.

use gafira_chart::{Piece, draw, parts};
use gafira_color::{Color, Rgba};
use gafira_doc::{ChartKind, ChartLabels, ChartNode, ChartText, NodeKind, Placement};
use kurbo::{Rect, Shape as _};
use proptest::prelude::*;

const KINDS: [ChartKind; 28] = [
    ChartKind::Columns,
    ChartKind::Rows,
    ChartKind::GroupedColumns,
    ChartKind::GroupedRows,
    ChartKind::StackedColumns,
    ChartKind::StackedRows,
    ChartKind::Line,
    ChartKind::Pie,
    ChartKind::Donut,
    ChartKind::StackedArea,
    ChartKind::Area,
    ChartKind::Stream,
    ChartKind::PercentArea,
    ChartKind::Scatter,
    ChartKind::Bubbles,
    ChartKind::DotColumns,
    ChartKind::DotRows,
    ChartKind::Pack,
    ChartKind::Treemap,
    ChartKind::NestedPack,
    ChartKind::Sunburst,
    ChartKind::ProgressRing,
    ChartKind::ProgressArc,
    ChartKind::ProgressBar,
    ChartKind::Gauge,
    ChartKind::Histogram,
    ChartKind::Funnel,
    ChartKind::Radar,
];

fn gray(v: u8) -> Color {
    let v = f32::from(v) / 255.0;
    Color::Srgb(Rgba::new(v, v, v, 1.0))
}

fn chart(kind: ChartKind, columns: usize, cells: &[&str]) -> ChartNode {
    let text = ChartText {
        font: "Inter".into(),
        size: 14.0,
        color: gray(0),
        bold: false,
        italic: false,
    };
    let colors = (1..=6).map(|i| gray(i * 30)).collect();
    ChartNode::new(
        kind,
        (columns, cells.iter().map(|c| (*c).to_owned()).collect()),
        colors,
        text,
    )
}

fn measure(text: &str, size: f64, _bold: bool) -> (f64, f64) {
    (text.chars().count() as f64 * size * 0.55, size * 1.2)
}

fn fills(chart: &ChartNode, size: (f64, f64)) -> Vec<(Rect, f64, Color)> {
    draw(chart, size, &measure)
        .into_iter()
        .filter_map(|p| match p {
            Piece::Fill { path, color } => Some((path.bounding_box(), path.area().abs(), color)),
            Piece::Words(_) => None,
        })
        .collect()
}

fn words(chart: &ChartNode) -> Vec<String> {
    draw(chart, (600.0, 400.0), &measure)
        .into_iter()
        .filter_map(|p| match p {
            Piece::Words(w) => Some(w.text),
            Piece::Fill { .. } => None,
        })
        .collect()
}

fn of(chart: &ChartNode, color: Color) -> Vec<(Rect, f64)> {
    fills(chart, (600.0, 400.0))
        .into_iter()
        .filter(|f| f.2 == color)
        .map(|f| (f.0, f.1))
        .collect()
}

const TEAMS: [&str; 10] = [
    "Equipo",
    "Cantidad",
    "Ventas",
    "110",
    "Finanzas",
    "50",
    "Marketing",
    "30",
    "RR. HH.",
    "10",
];

// S83-03: bars as long as their values, from zero, the negative ones too.
#[test]
fn s83_03_bars() {
    let c = chart(
        ChartKind::Columns,
        2,
        &["", "v", "a", "10", "b", "20", "c", "-10"],
    );
    let bars = of(&c, gray(30));
    assert_eq!(bars.len(), 3);
    assert!((bars[1].0.height() - 2.0 * bars[0].0.height()).abs() < 1e-6);
    assert!(
        (bars[0].0.y1 - bars[2].0.y0).abs() < 1e-6,
        "one above zero, one below"
    );
    let c = chart(ChartKind::Rows, 2, &["", "v", "a", "10", "b", "20"]);
    let bars = of(&c, gray(30));
    assert!((bars[1].0.width() - 2.0 * bars[0].0.width()).abs() < 1e-6);
    assert!(bars[0].0.y0 < bars[1].0.y0, "the first one above");
    let mut c = chart(ChartKind::Columns, 2, &TEAMS);
    c.by_label = true;
    assert_eq!(of(&c, gray(120)).len(), 1, "a colour each");
}

// S83-03: the parts of a stack touch or part by the same gap; groups do not
// cover each other.
#[test]
fn s83_03_stacks_and_groups() {
    let cells = ["", "a", "b", "x", "10", "30", "y", "20", "20"];
    let c = chart(ChartKind::StackedColumns, 3, &cells);
    let (a, b) = (of(&c, gray(30)), of(&c, gray(60)));
    assert!((a[0].0.height() + b[0].0.height() - a[1].0.height() - b[1].0.height()).abs() < 0.1);
    assert!(b[0].0.y1 <= a[0].0.y0 + 1e-6, "the second over the first");
    let c = chart(ChartKind::GroupedColumns, 3, &cells);
    let (a, b) = (of(&c, gray(30)), of(&c, gray(60)));
    assert!(a[0].0.x1 <= b[0].0.x0 + 1e-6 && b[0].0.x1 <= a[1].0.x0);
}

// S83-03: slices as wide as their share; a lone one is whole.
#[test]
fn s83_03_slices() {
    let c = chart(ChartKind::Pie, 2, &TEAMS);
    let area: Vec<f64> = (1..=4).map(|i| of(&c, gray(i * 30))[0].1).collect();
    let whole: f64 = area.iter().sum();
    for (area, share) in area.iter().zip([0.55, 0.25, 0.15, 0.05]) {
        assert!((area / whole - share).abs() < 0.01, "{area} {share}");
    }
    let one = chart(ChartKind::Donut, 2, &["", "v", "a", "3"]);
    let (rect, ring) = of(&one, gray(30))[0];
    let disc = std::f64::consts::PI * rect.width() * rect.width() / 4.0;
    assert!((ring / disc - (1.0 - 0.36)).abs() < 0.02, "{}", ring / disc);
    let negative = chart(ChartKind::Pie, 2, &["", "v", "a", "3", "b", "-1"]);
    assert!(of(&negative, gray(60)).is_empty());
}

// S83-03: the cells of a tree map, the circles and the bubbles have areas in
// proportion, and none lies over another.
#[test]
fn s83_03_areas() {
    let c = chart(ChartKind::Treemap, 2, &TEAMS);
    let cells: Vec<(Rect, f64)> = (1..=4).map(|i| of(&c, gray(i * 30))[0]).collect();
    assert!((cells[0].1 / cells[1].1 - 2.2).abs() < 0.1);
    for (i, a) in cells.iter().enumerate() {
        for b in &cells[i + 1..] {
            assert!(a.0.intersect(b.0).area() < 1e-6);
        }
    }
    let c = chart(ChartKind::Pack, 2, &TEAMS);
    let circles: Vec<(Rect, f64)> = (1..=4).map(|i| of(&c, gray(i * 30))[0]).collect();
    assert!((circles[0].1 / circles[1].1 - 2.2).abs() < 0.05);
    for (i, a) in circles.iter().enumerate() {
        for b in &circles[i + 1..] {
            let apart = a.0.center().distance(b.0.center());
            assert!(apart >= (a.0.width() + b.0.width()) / 2.0 - 1e-3);
        }
    }
}

// S83-03: progress against a total, a percentage or a hundred.
#[test]
fn s83_03_progress() {
    let share = |cells: &[&str], columns| {
        let c = chart(ChartKind::ProgressBar, columns, cells);
        let all = fills(&c, (600.0, 400.0));
        let full = all.iter().map(|f| f.0.width()).fold(0.0, f64::max);
        of(&c, gray(30))[0].0.width() / full
    };
    assert!((share(&["", "v", "t", "a", "30", "40"], 3) - 0.75).abs() < 0.02);
    assert!((share(&["", "v", "a", "75%"], 2) - 0.75).abs() < 0.02);
    assert!((share(&["", "v", "a", "75"], 2) - 0.75).abs() < 0.02);
    assert!(
        (share(&["", "v", "a", "500"], 2) - 1.0).abs() < 0.02,
        "no more than whole"
    );
    let mut c = chart(ChartKind::ProgressRing, 2, &["", "v", "a", "75%"]);
    c.numbers = ChartLabels::Percent;
    assert!(words(&c).contains(&"75%".to_owned()));
}

// S83-03: the words: labels, values as they were written, shares, title.
#[test]
fn s83_03_words() {
    let mut c = chart(ChartKind::Pie, 2, &TEAMS);
    c.names = true;
    c.numbers = ChartLabels::Percent;
    c.title = "Personal".into();
    let all = words(&c).join("|");
    for word in ["Personal", "Ventas", "55%", "RR. HH.", "5%"] {
        assert!(all.contains(word), "{word}: {all}");
    }
    let mut c = chart(ChartKind::Columns, 2, &["", "v", "a", "$ 1.200,50"]);
    c.numbers = ChartLabels::Value;
    c.axis = true;
    c.names = true;
    let all = words(&c);
    assert!(
        all.contains(&"$ 1.200,50".to_owned()) && all.contains(&"a".to_owned()),
        "{all:?}"
    );
    assert!(all.contains(&"0".to_owned()), "the axis: {all:?}");
    c.numbers = ChartLabels::None;
    c.axis = false;
    c.names = false;
    assert!(words(&c).is_empty());
}

// S83-05: two series or more have a legend; a lone one does not.
#[test]
fn s83_05_legend() {
    let mut c = chart(
        ChartKind::Line,
        3,
        &["", "Uno", "Dos", "a", "1", "2", "b", "3", "4"],
    );
    c.legend = true;
    let all = words(&c);
    assert!(all.contains(&"Uno".to_owned()) && all.contains(&"Dos".to_owned()));
    let mut c = chart(ChartKind::Line, 2, &["", "Uno", "a", "1", "b", "3"]);
    c.legend = true;
    assert!(!words(&c).contains(&"Uno".to_owned()));
}

// S83-05: the words over a colour stand out.
#[test]
fn s83_05_words_over_colours() {
    for tone in [0, 255] {
        let mut c = chart(ChartKind::Treemap, 2, &["", "v", "Uno", "3"]);
        c.colors = vec![gray(tone)];
        c.names = true;
        let word = draw(&c, (600.0, 400.0), &measure)
            .into_iter()
            .find_map(|p| match p {
                Piece::Words(w) => Some(w.color),
                Piece::Fill { .. } => None,
            });
        let Some(Color::Srgb(word)) = word else {
            panic!("a word")
        };
        assert!((word.r - f32::from(tone) / 255.0).abs() > 0.5);
    }
}

// S83-03: nothing to draw, or nowhere to, draws nothing wrong.
#[test]
fn s83_03_nothing() {
    for kind in KINDS {
        for cells in [
            &["", ""][..],
            &["a", "b", "", ""],
            &["a", "b", "x", "y"],
            &["a", "b", "x", "0"],
            &["a", "b", "x", "-5"],
        ] {
            let mut c = chart(kind, 2, cells);
            (c.names, c.axis, c.grid, c.legend, c.markers) = (true, true, true, true, true);
            for size in [
                (600.0, 400.0),
                (1.0, 1.0),
                (0.0, 0.0),
                (5000.0, 3.0),
                (f64::NAN, 5.0),
            ] {
                check(&c, size);
            }
        }
    }
}

fn check(c: &ChartNode, size: (f64, f64)) {
    for piece in draw(c, size, &measure) {
        match piece {
            Piece::Fill { path, .. } => assert!(path.is_finite(), "{:?} {size:?}", c.kind),
            Piece::Words(w) => {
                assert!(
                    w.at.is_finite() && w.size > 0.0 && w.size.is_finite(),
                    "{:?}",
                    c.kind
                );
                assert!(!w.text.trim().is_empty());
            }
        }
    }
}

// S83-11 (the size of what is drawn): every kind draws its sample inside
// its box.
#[test]
fn s83_03_every_kind_draws_inside_its_box() {
    let cells = [
        "Zona", "Equipo", "A", "B", "C", "Norte", "Uno", "10", "4", "7", "Norte", "Dos", "6", "8",
        "2", "Sur", "Tres", "9", "5", "6", "Sur", "Cuatro", "3", "7", "9",
    ];
    for kind in KINDS {
        let mut c = chart(kind, 5, &cells);
        (c.names, c.axis, c.grid, c.legend, c.markers, c.smooth) =
            (true, true, true, true, true, true);
        c.numbers = ChartLabels::Value;
        c.title = "Título".into();
        let all = draw(&c, (600.0, 400.0), &measure);
        let box_of = Rect::new(-3.0, -3.0, 603.0, 403.0);
        let mut filled = 0;
        for piece in &all {
            if let Piece::Fill { path, .. } = piece {
                let b = path.bounding_box();
                assert!(box_of.union(b) == box_of, "{kind:?} {b:?}");
                filled += 1;
            }
        }
        assert!(filled > 0, "{kind:?}");
    }
}

// S83-09: the same drawing as shapes and texts for PowerPoint.
#[test]
fn s83_09_parts() {
    let mut c = chart(ChartKind::Pie, 2, &TEAMS);
    c.names = true;
    let all = parts(&c, &Placement::new(50.0, 60.0, 600.0, 400.0));
    let shapes = all
        .iter()
        .filter(|p| matches!(p.0, NodeKind::Shape(_)))
        .count();
    let texts = all
        .iter()
        .filter(|p| matches!(p.0, NodeKind::Text(_)))
        .count();
    assert!(shapes >= 4 && texts >= 4, "{shapes} {texts}");
    for (_, p) in &all {
        assert!(
            p.x >= 0.0 && p.w > 0.0 && p.h > 0.0 && p.x + p.w <= 700.0,
            "{p:?}"
        );
    }
}

proptest! {
    // S83-03: any data, any kind, any box.
    #[test]
    fn s83_03_any_data(
        kind in 0_usize..28,
        cells in prop::collection::vec("(-?[0-9]{1,15}|[0-9]{1,3}%|[a-c]|)", 12),
        columns in prop::sample::select(vec![1_usize, 2, 3, 4, 6]),
        w in 0.0..900.0_f64, h in 0.0..900.0_f64, size in 1.0..200.0_f64, flags in any::<u8>(),
    ) {
        let cells: Vec<&str> = cells.iter().map(String::as_str).collect();
        let mut c = chart(KINDS[kind], columns, &cells);
        c.text.size = size;
        (c.names, c.axis, c.grid, c.legend) = (flags & 1 > 0, flags & 2 > 0, flags & 4 > 0, flags & 8 > 0);
        (c.markers, c.smooth, c.by_label) = (flags & 16 > 0, flags & 32 > 0, flags & 64 > 0);
        c.numbers = if flags & 128 > 0 { ChartLabels::Percent } else { ChartLabels::Value };
        c.round = 1.0;
        check(&c, (w, h));
    }
}
