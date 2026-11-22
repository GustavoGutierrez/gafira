//! S83: the data of a chart and their changes.

use gafira_chart::{
    Colored, Column, color_names, colored, column, columns, from_grid, grid, is_number, places,
    read, set_cell, ticks, transpose,
};
use gafira_color::{Color, Rgba};
use gafira_doc::{ChartCombine, ChartKind, ChartNode, ChartText};
use proptest::prelude::*;

fn chart(kind: ChartKind, columns: usize, cells: &[&str]) -> ChartNode {
    let ink = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0));
    let text = ChartText {
        font: "Inter".into(),
        size: 14.0,
        color: ink,
        bold: false,
        italic: false,
    };
    ChartNode::new(
        kind,
        (columns, cells.iter().map(|c| (*c).to_owned()).collect()),
        vec![ink],
        text,
    )
}

const SALES: [&str; 12] = [
    "Mes", "Ventas", "Nota", "Meta", "Ene", "10", "x", "12,5", "Feb", "-3", "", "20",
];

// S83-02: words, numbers and nothing; a wrong cell does not spoil a column.
#[test]
fn s83_02_columns() {
    let c = chart(ChartKind::Columns, 4, &SALES);
    assert_eq!(
        (0..4).map(|i| column(&c, i)).collect::<Vec<_>>(),
        [
            Column::Words,
            Column::Numbers,
            Column::Words,
            Column::Numbers
        ]
    );
    let c = chart(
        ChartKind::Columns,
        2,
        &["a", "b", "1", "", "dos", "", "3", ""],
    );
    assert_eq!(
        (column(&c, 0), column(&c, 1)),
        (Column::Numbers, Column::Empty)
    );
    let data = read(&c);
    assert_eq!(data.series[0].values, [Some(1.0), None, Some(3.0)]);
    assert_eq!(column(&c, 9), Column::Empty);
}

// S83-02: which columns are drawn: found, or chosen.
#[test]
fn s83_02_roles() {
    let c = chart(ChartKind::GroupedColumns, 4, &SALES);
    assert_eq!(columns(&c), (Some(0), vec![1, 3]));
    // A pie has one series.
    let mut c = chart(ChartKind::Pie, 4, &SALES);
    assert_eq!(columns(&c), (Some(0), vec![1]));
    c.labels = Some(2);
    c.values = vec![3];
    let data = read(&c);
    assert_eq!(
        (data.labels, data.series[0].values.clone()),
        (
            vec!["x".to_owned(), String::new()],
            vec![Some(12.5), Some(20.0)]
        )
    );
    assert_eq!(
        (
            data.series[0].name.as_str(),
            data.series[0].words[0].as_str(),
            data.comma
        ),
        ("Meta", "12,5", true)
    );
    // Two columns of numbers: an x and a y, with no label.
    let c = chart(ChartKind::Scatter, 2, &["x", "y", "1", "2"]);
    assert_eq!(columns(&c), (None, vec![0, 1]));
    // Years name the rows when there are no words.
    let c = chart(
        ChartKind::Line,
        2,
        &["Año", "Total", "2023", "5", "2024", "6"],
    );
    assert_eq!(columns(&c), (Some(0), vec![1]));
    // One column of numbers is the values.
    let c = chart(ChartKind::Columns, 1, &["Total", "5"]);
    assert_eq!(columns(&c), (None, vec![0]));
}

// S83-02: empty rows are left out; the same label is combined.
#[test]
fn s83_02_combine() {
    let cells = [
        "Zona", "Total", "A", "10", "B", "1", "", "", "A", "20", "A", "x",
    ];
    let mut c = chart(ChartKind::Columns, 2, &cells);
    assert_eq!(read(&c).labels, ["A", "B", "A", "A"]);
    let mut values = |how| {
        c.combine = how;
        let data = read(&c);
        assert_eq!(data.labels, ["A", "B"]);
        data.series[0].values.clone()
    };
    assert_eq!(values(ChartCombine::Sum), [Some(30.0), Some(1.0)]);
    assert_eq!(values(ChartCombine::Mean), [Some(15.0), Some(1.0)]);
    assert_eq!(values(ChartCombine::Min), [Some(10.0), Some(1.0)]);
    assert_eq!(values(ChartCombine::Max), [Some(20.0), Some(1.0)]);
    assert_eq!(values(ChartCombine::Count), [Some(2.0), Some(1.0)]);
}

// S83-05: what each kind colours.
#[test]
fn s83_05_colored() {
    let kind = |kind, by_label| {
        let mut c = chart(kind, 4, &SALES);
        c.by_label = by_label;
        colored(&c)
    };
    assert_eq!(kind(ChartKind::Columns, false), Colored::Series);
    assert_eq!(kind(ChartKind::Columns, true), Colored::Rows);
    assert_eq!(kind(ChartKind::GroupedRows, true), Colored::Series);
    assert_eq!(kind(ChartKind::Donut, false), Colored::Rows);
    assert_eq!(kind(ChartKind::Sunburst, false), Colored::Groups);
    assert_eq!(kind(ChartKind::Bubbles, false), Colored::Groups);
}

// S83-07: the colour of each row in the editor is the one it is drawn with.
#[test]
fn s83_07_places_and_names() {
    let c = chart(ChartKind::Pie, 2, &["a", "v", "x", "1", "", "", "y", "2"]);
    assert_eq!(places(&c), [None, Some(0), None, Some(1)]);
    assert_eq!(color_names(&c), ["x", "y"]);
    let c = chart(ChartKind::Line, 4, &SALES);
    assert_eq!(color_names(&c), ["Ventas", "Meta"]);
    let c = chart(
        ChartKind::Bubbles,
        4,
        &[
            "g", "x", "y", "z", "A", "1", "2", "3", "A", "1", "2", "3", "B", "1", "2", "3",
        ],
    );
    assert_eq!(color_names(&c), ["A", "B"]);
    assert!(is_number("12,5") && !is_number("1.234") && !is_number("a"));
}

// S83-07: cells, rows and columns; what is chosen follows.
#[test]
fn s83_07_edits() {
    let mut c = chart(ChartKind::Columns, 2, &["a", "b", "c", "1"]);
    assert!(set_cell(&mut c, 1, 1, "2"));
    assert!(!set_cell(&mut c, 1, 1, "2"), "nothing changes");
    // Writing past the end makes room for it.
    assert!(set_cell(&mut c, 3, 2, "z"));
    assert_eq!(
        (c.columns, c.rows(), c.cell(1, 1), c.cell(3, 2)),
        (3, 4, "2", "z")
    );
    // Emptying it takes the room back.
    assert!(set_cell(&mut c, 3, 2, ""));
    assert_eq!((c.columns, c.rows()), (2, 2));
    assert!(!set_cell(&mut c, 201, 0, "x") && !set_cell(&mut c, 0, 20, "x"));
    assert!(!set_cell(&mut c, 0, 0, &"x".repeat(1025)));
    c.labels = Some(1);
    c.values = vec![0];
    assert!(transpose(&mut c));
    assert_eq!(grid(&c), [["a", "c"], ["b", "2"]]);
    assert_eq!(
        (c.labels, c.values.len()),
        (None, 0),
        "the columns are others"
    );
    assert!(from_grid(
        &mut c,
        &[vec!["n".into()], vec!["1".into(), "2".into()]]
    ));
    assert_eq!(grid(&c), [["n", ""], ["1", "2"]]);
    assert!(!from_grid(&mut c, &[]) && c.is_valid());
    let tall = vec![vec!["1".to_owned(); 30]; 300];
    assert!(from_grid(&mut c, &tall));
    assert_eq!(
        (c.rows(), c.columns),
        (ChartNode::MAX_ROWS, ChartNode::MAX_COLUMNS)
    );
    let mut wide = chart(ChartKind::Columns, 1, &["x"; 60]);
    assert!(!transpose(&mut wide), "more columns than a chart holds");
}

// S83-04: round steps that cover what they are asked to.
#[test]
fn s83_04_ticks() {
    assert_eq!(ticks(0.0, 100.0, 5), [0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
    assert_eq!(ticks(0.0, 7.0, 5), [0.0, 2.0, 4.0, 6.0, 8.0]);
    assert_eq!(ticks(-5.0, 5.0, 5), [-6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0]);
    assert_eq!(ticks(0.0, 0.3, 3), [0.0, 0.1, 0.2, 0.3]);
    assert_eq!(ticks(4.0, 4.0, 5).first(), Some(&4.0), "a lone value");
    assert_eq!(ticks(f64::NAN, 1.0, 5), [0.0, 1.0]);
}

proptest! {
    #[test]
    fn s83_04_ticks_cover(a in -1e9..1e9_f64, len in 1e-6..1e9_f64, most in 2_usize..12) {
        let t = ticks(a, a + len, most);
        prop_assert!(t.len() >= 2 && t.len() <= 2 * most + 3, "{}", t.len());
        prop_assert!(t[0] <= a + len * 1e-6 && *t.last().unwrap() >= a + len * (1.0 - 1e-6));
        prop_assert!(t.windows(2).all(|w| w[0] < w[1]));
    }

    // S83-07: any change leaves a chart a document holds.
    #[test]
    fn s83_07_edits_stay_valid(edits in prop::collection::vec((0_usize..210, 0_usize..24, "[a-z0-9]{0,3}"), 0..20), turn in any::<bool>()) {
        let mut c = chart(ChartKind::Line, 2, &["a", "b", "c", "1"]);
        c.labels = Some(1);
        c.values = vec![0, 1];
        for (row, col, words) in edits {
            set_cell(&mut c, row, col, &words);
            prop_assert!(c.is_valid());
        }
        if turn { transpose(&mut c); }
        prop_assert!(c.is_valid());
    }
}
