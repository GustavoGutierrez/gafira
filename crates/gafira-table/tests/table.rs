//! Layout, changes, borders, clipboard and values of tables (spec 081).

use gafira_color::{Color, Rgba};
use gafira_doc::{CellMerge, CellStyle, ShapeStroke, TableNode};
use gafira_table::*;
use kurbo::Rect;
use proptest::prelude::*;

fn ink(v: f32) -> Color {
    Color::Srgb(Rgba::new(v, v, v, 1.0))
}

/// A table whose cells say their row and column: `00`, `01`…
fn table(rows: usize, cols: usize) -> TableNode {
    let mut t = TableNode::new(
        rows,
        cols,
        (100.0, 40.0),
        &CellStyle::new("Inter", 16.0, ink(0.0)),
    );
    for r in 0..rows {
        for c in 0..cols {
            t.cell_mut(r, c).expect("a cell").text = format!("{r}{c}");
        }
    }
    t
}

fn texts(t: &TableNode) -> Vec<Vec<&str>> {
    t.cells
        .chunks(t.columns.len())
        .map(|row| row.iter().map(|c| c.text.as_str()).collect())
        .collect()
}

fn at(row: usize, col: usize) -> Range {
    Range::cell(row, col)
}

fn joined(row: usize, col: usize, rows: usize, cols: usize) -> CellMerge {
    CellMerge {
        row,
        col,
        rows,
        cols,
    }
}

// S81-03: the sizes are shares of the box, with room between the cells.
#[test]
fn s81_03_layout_fills_the_box() {
    let mut t = table(2, 2);
    t.columns = vec![1.0, 3.0];
    let l = Layout::of(&t, (200.0, 100.0));
    assert_eq!(l.cell(&t, 0, 0), Rect::new(0.0, 0.0, 50.0, 50.0));
    assert_eq!(l.cell(&t, 1, 1), Rect::new(50.0, 50.0, 200.0, 100.0));
    t.spacing = 20.0;
    let l = Layout::of(&t, (220.0, 120.0));
    assert_eq!(l.cell(&t, 0, 0), Rect::new(0.0, 0.0, 50.0, 50.0));
    assert_eq!(l.cell(&t, 1, 1), Rect::new(70.0, 70.0, 220.0, 120.0));
    assert_eq!(l.at(&t, 100.0, 10.0), Some((0, 1)));
    assert_eq!(l.at(&t, 230.0, 10.0), None);
    t.merges.push(joined(0, 0, 2, 2));
    assert_eq!(l.cell(&t, 0, 0), Rect::new(0.0, 0.0, 220.0, 120.0));
    assert_eq!(l.at(&t, 100.0, 100.0), Some((0, 0)), "the merge's own cell");
    assert_eq!(anchors(&t).collect::<Vec<_>>(), vec![(0, 0)]);
}

// S81-03: the borders between columns and rows are grabbed near them.
#[test]
fn s81_03_the_edges_are_grabbed() {
    let t = table(2, 2);
    let l = Layout::of(&t, (200.0, 80.0));
    assert_eq!(l.column_edge(103.0, 4.0), Some(0));
    assert_eq!(l.column_edge(198.0, 4.0), Some(1));
    assert_eq!(l.column_edge(50.0, 4.0), None);
    assert_eq!(l.column_edge(0.0, 4.0), None, "the left side is not one");
    assert_eq!(l.row_edge(41.0, 4.0), Some(0));
}

// S81-03: a size changes alone, and never below the least.
#[test]
fn s81_03_a_column_is_resized_alone() {
    let mut t = table(2, 2);
    to_px(&mut t, (400.0, 80.0));
    assert_eq!(t.columns, vec![200.0, 200.0]);
    resize_column(&mut t, 0, 30.0);
    assert_eq!(t.columns, vec![230.0, 200.0]);
    resize_column(&mut t, 1, -900.0);
    assert_eq!(t.columns, vec![230.0, MIN_SIZE]);
    resize_row(&mut t, 1, 10.0);
    assert_eq!(t.rows, vec![40.0, 50.0]);
    t.spacing = 5.0;
    assert_eq!(size(&t), (230.0 + MIN_SIZE + 5.0, 95.0));
}

// S81-02: rows and columns come and go; a new one takes the looks of the
// one beside it and no words.
#[test]
fn s81_02_rows_and_columns() {
    let mut t = table(2, 2);
    t.cell_mut(1, 0).expect("a cell").style.bold = true;
    assert!(insert_row(&mut t, 2, 1));
    assert_eq!(texts(&t), [["00", "01"], ["10", "11"], ["", ""]]);
    assert!(t.cell(2, 0).expect("a cell").style.bold);
    assert_eq!(t.rows, vec![40.0; 3]);
    assert!(insert_column(&mut t, 0, 0));
    assert_eq!(texts(&t)[1], ["", "10", "11"]);
    assert!(remove_rows(&mut t, 0, 0));
    assert!(remove_columns(&mut t, 0, 1));
    assert_eq!(texts(&t), [["11"], [""]]);
    assert!(!remove_columns(&mut t, 0, 0), "never the last one");
    assert!(duplicate_rows(&mut t, 0, 0));
    assert_eq!(texts(&t), [["11"], ["11"], [""]]);
    assert!(move_row(&mut t, 2, false));
    assert_eq!(texts(&t), [["11"], [""], ["11"]]);
    assert!(!move_row(&mut t, 2, true), "past the end");
    let mut wide = table(1, TableNode::MAX_COLUMNS);
    assert!(!insert_column(&mut wide, 0, 0), "the limit");
    let mut pair = table(1, 2);
    assert!(move_column(&mut pair, 0, true));
    assert_eq!(texts(&pair), [["01", "00"]]);
}

// S81-04: cells join and part; their words are kept, one under another.
#[test]
fn s81_04_merging() {
    let mut t = table(3, 3);
    t.cell_mut(0, 1).expect("a cell").text.clear();
    assert!(!merge(&mut t, at(0, 0)), "one cell is nothing to join");
    assert!(merge(&mut t, Range::new((0, 0), (1, 1))));
    assert_eq!(t.merges, vec![joined(0, 0, 2, 2)]);
    assert_eq!(t.cell(0, 0).expect("a cell").text, "00\n10\n11");
    assert_eq!(t.cell(1, 1).expect("a cell").text, "");
    // A range takes the whole of what it touches.
    assert_eq!(
        Range::new((1, 1), (2, 1)).over(&t),
        Range::new((0, 0), (2, 1))
    );
    assert!(merge(&mut t, Range::new((1, 1), (2, 1))));
    assert_eq!(t.merges, vec![joined(0, 0, 3, 2)]);
    assert!(unmerge(&mut t, at(0, 0)));
    assert!(t.merges.is_empty() && t.is_valid());
    assert!(!unmerge(&mut t, at(0, 0)));
}

// S81-04: merges follow the rows and columns around and inside them.
#[test]
fn s81_04_merges_follow_their_rows() {
    let start = || {
        let mut t = table(4, 2);
        merge(&mut t, Range::new((1, 0), (2, 0)));
        t
    };
    let mut t = start();
    insert_row(&mut t, 0, 0);
    assert_eq!(t.merges, vec![joined(2, 0, 2, 1)], "moved down");
    let mut t = start();
    insert_row(&mut t, 2, 1);
    assert_eq!(t.merges, vec![joined(1, 0, 3, 1)], "grown");
    let mut t = start();
    insert_row(&mut t, 1, 0);
    assert_eq!(t.merges, vec![joined(2, 0, 2, 1)], "before it");
    let mut t = start();
    remove_rows(&mut t, 1, 1);
    assert!(t.merges.is_empty(), "one cell is left");
    assert_eq!(
        t.cell(1, 0).expect("a cell").text,
        "10\n20",
        "its words stay"
    );
    let mut t = start();
    remove_rows(&mut t, 0, 0);
    assert_eq!(t.merges, vec![joined(0, 0, 2, 1)]);
    let mut t = start();
    let before = t.clone();
    assert!(!move_row(&mut t, 2, true), "it would be torn");
    assert_eq!(t, before);
}

proptest! {
    // S81-02, S81-04: after any changes the table is one a document holds.
    #[test]
    fn s81_02_any_changes_leave_a_valid_table(
        steps in proptest::collection::vec((0_u8..12, 0_usize..6, 0_usize..6, 0_usize..6, 0_usize..6), 1..40)
    ) {
        let mut t = table(4, 4);
        for (what, a, b, c, d) in steps {
            let (rows, cols) = (t.rows.len(), t.columns.len());
            let range = Range::new((a % rows, b % cols), (c % rows, d % cols));
            match what {
                0 => drop(insert_row(&mut t, a % (rows + 1), b % rows)),
                1 => drop(insert_column(&mut t, a % (cols + 1), b % cols)),
                2 => drop(remove_rows(&mut t, range.r0, range.r1)),
                3 => drop(remove_columns(&mut t, range.c0, range.c1)),
                4 | 5 => drop(merge(&mut t, range)),
                6 => drop(unmerge(&mut t, range)),
                7 => drop(move_row(&mut t, a % rows, b % 2 == 0)),
                8 => drop(move_column(&mut t, a % cols, b % 2 == 0)),
                9 => drop(duplicate_rows(&mut t, range.r0, range.r1)),
                10 => drop(paste(&mut t, (a % rows, b % cols), &[vec!["x".to_owned(); 3], vec!["y".to_owned(); 2]])),
                _ => set_borders(&mut t, range, BorderSet::All, Some(ShapeStroke::new(ink(0.0), 1.0))),
            }
            prop_assert!(t.is_valid(), "after {what}");
        }
    }
}

// S81-02: sizes made alike.
#[test]
fn s81_02_sizes_are_made_alike() {
    let mut t = table(2, 3);
    t.columns = vec![10.0, 20.0, 60.0];
    equalize_columns(&mut t, 0, 2);
    assert_eq!(t.columns, vec![30.0; 3]);
    t.rows = vec![10.0, 30.0];
    equalize_rows(&mut t, 0, 1);
    assert_eq!(t.rows, vec![20.0, 20.0]);
}

fn pen() -> ShapeStroke {
    ShapeStroke::new(ink(0.0), 2.0)
}

/// The borders drawn, as (x0, y0, x1, y1) without the joins of the corners.
fn drawn(t: &TableNode) -> Vec<(f64, f64, f64, f64)> {
    let l = Layout::of(t, size(t));
    let mut out: Vec<_> = edges(t, &l)
        .iter()
        .map(|e| (e.a.x, e.a.y, e.b.x, e.b.y))
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).expect("numbers"));
    out
}

// S81-05: each set draws its own edges, joined along a line.
#[test]
fn s81_05_border_sets() {
    let all = Range::new((0, 0), (1, 1));
    let of = |which| {
        let mut t = table(2, 2);
        set_borders(&mut t, all, which, Some(pen()));
        drawn(&t)
    };
    let (h, v) = (|y: f64| (0.0, y, 200.0, y), |x: f64| (x, 0.0, x, 80.0));
    assert_eq!(of(BorderSet::Top), vec![h(0.0)]);
    assert_eq!(of(BorderSet::Horizontal), vec![h(40.0)]);
    assert_eq!(of(BorderSet::Bottom), vec![h(80.0)]);
    assert_eq!(of(BorderSet::Left), vec![v(0.0)]);
    assert_eq!(of(BorderSet::Vertical), vec![v(100.0)]);
    assert_eq!(of(BorderSet::Right), vec![v(200.0)]);
    assert_eq!(of(BorderSet::Inner), vec![h(40.0), v(100.0)]);
    assert_eq!(of(BorderSet::Outer).len(), 4);
    assert_eq!(of(BorderSet::All).len(), 6);
    let mut t = table(2, 2);
    set_borders(&mut t, all, BorderSet::All, Some(pen()));
    set_borders(&mut t, all, BorderSet::All, None);
    assert!(drawn(&t).is_empty());
}

// S81-05: the edge two cells share is one: either of them takes it off.
#[test]
fn s81_05_a_shared_edge_is_one() {
    let mut t = table(1, 2);
    set_borders(&mut t, at(0, 0), BorderSet::Right, Some(pen()));
    assert_eq!(drawn(&t), vec![(100.0, 0.0, 100.0, 40.0)]);
    set_borders(&mut t, at(0, 1), BorderSet::Left, None);
    assert!(drawn(&t).is_empty());
    // Inside a merge there is none.
    set_borders(
        &mut t,
        Range::new((0, 0), (0, 1)),
        BorderSet::All,
        Some(pen()),
    );
    merge(&mut t, Range::new((0, 0), (0, 1)));
    assert_eq!(drawn(&t).len(), 4);
    // With room between the cells each one has its own.
    let mut apart = table(1, 2);
    apart.spacing = 10.0;
    set_borders(&mut apart, at(0, 0), BorderSet::Right, Some(pen()));
    assert_eq!(drawn(&apart), vec![(100.0, 0.0, 100.0, 40.0)]);
}

// S81-06: looks go to a range, and from a cell to others.
#[test]
fn s81_06_looks() {
    let mut t = table(2, 2);
    restyle(&mut t, Range::new((0, 0), (0, 1)), |s| {
        s.fill = Some(ink(0.5))
    });
    assert_eq!(t.cell(0, 1).expect("a cell").style.fill, Some(ink(0.5)));
    assert_eq!(t.cell(1, 0).expect("a cell").style.fill, None);
    let style = t.cell(0, 0).expect("a cell").style.clone();
    paste_style(&mut t, at(1, 1), &style);
    assert_eq!(t.cell(1, 1).expect("a cell").style, style);
    assert_eq!(t.cell(1, 1).expect("a cell").text, "11");
    clear(&mut t, Range::new((0, 0), (1, 1)));
    assert!(t.cells.iter().all(|c| c.text.is_empty()));
}

// S81-11, FD-21 a: what a spreadsheet copies fills the cells, quotes and
// line breaks in them too; the table grows to take it.
#[test]
fn s81_11_the_clipboard() {
    let grid = parse_grid("a\tb\tc\r\n\"x\ny\"\t\"say \"\"hi\"\"\"\t\r\n1\t2\t3\r\n");
    assert_eq!(
        grid,
        vec![
            vec!["a", "b", "c"],
            vec!["x\ny", "say \"hi\"", ""],
            vec!["1", "2", "3"]
        ]
    );
    let mut t = table(2, 2);
    assert!(paste(&mut t, (0, 0), &grid));
    assert_eq!((t.rows.len(), t.columns.len()), (3, 3));
    assert_eq!(texts(&t)[1], ["x\ny", "say \"hi\"", ""]);
    let all = Range::new((0, 0), (2, 2));
    assert_eq!(
        copy(&t, all),
        "a\tb\tc\n\"x\ny\"\t\"say \"\"hi\"\"\"\t\n1\t2\t3"
    );
    assert_eq!(parse_grid(&copy(&t, all)), grid, "there and back");
    assert_eq!(parse_grid("solo"), vec![vec!["solo"]]);
    assert_eq!(parse_grid("5\" de alto"), vec![vec!["5\" de alto"]]);
    // Past the limits it fills what it can.
    let mut full = table(1, TableNode::MAX_COLUMNS);
    let last = TableNode::MAX_COLUMNS - 1;
    assert!(paste(&mut full, (0, last), &grid));
    assert_eq!(full.columns.len(), TableNode::MAX_COLUMNS);
    assert_eq!(full.cell(2, last).expect("a cell").text, "1");
}

// S81-14: a number is a number only when nothing is lost or guessed.
#[test]
fn s81_14_values() {
    use CellValue::{Empty, Number, Text};
    use NumberFormat::{Currency, General, Percent};
    let number = |v: f64, f: NumberFormat| Number(v, f);
    assert_eq!(value("  "), Empty);
    assert_eq!(value("12"), number(12.0, General));
    assert_eq!(value(" -3.5 "), number(-3.5, General));
    assert_eq!(value("3,5"), number(3.5, General));
    assert_eq!(value("1.234,56"), number(1234.56, General));
    assert_eq!(value("1,234.56"), number(1234.56, General));
    assert_eq!(value("1.234.567"), number(1_234_567.0, General));
    assert_eq!(value("15%"), number(0.15, Percent));
    assert_eq!(value("12,5 %"), number(0.125, Percent));
    assert_eq!(value("$ 1,250.00"), number(1250.0, Currency('$')));
    assert_eq!(value("30 €"), number(30.0, Currency('€')));
    // S82-01: more than three digits before it: it can only be a decimal sign.
    assert_eq!(value("1234.567"), number(1234.567, General));
    assert_eq!(value("12345,678"), number(12345.678, General));
    assert_eq!(value("-$30.00"), number(-30.0, Currency('$')));
    assert_eq!(value("$-30"), number(-30.0, Currency('$')));
    assert_eq!(value("-30 €"), number(-30.0, Currency('€')));
    for words in [
        "--5",
        "-$-5",
        "007",
        "1.234",
        "1,234",
        "+57 300",
        "=1+1",
        "12a",
        "1e5",
        "nan",
        "inf",
        "-",
        ".",
        "1.2.3",
        "12345678901234567",
        "1.23.456,7",
        "%",
        "$",
    ] {
        assert_eq!(value(words), Text, "{words}");
    }
    assert_eq!(value("0"), number(0.0, General));
    assert_eq!(value("0.5"), number(0.5, General));
}

// S81-15: what a chart needs of a table.
#[test]
fn s81_15_data() {
    let mut t = table(3, 3);
    for (i, words) in [
        "Mes", "Ventas", "Nota", "Enero", "10", "x", "Febrero", "12,5", "",
    ]
    .into_iter()
    .enumerate()
    {
        t.cells[i].text = words.to_owned();
    }
    t.header = true;
    let d = data(&t);
    assert_eq!(d.headers, ["Ventas"]);
    assert_eq!(d.categories, ["Enero", "Febrero"]);
    assert_eq!(d.columns, vec![vec![Some(10.0), Some(12.5)]]);
}
