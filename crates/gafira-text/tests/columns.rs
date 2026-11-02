//! FD-45: columns. Equal-width columns split a text by the height of the box, or balance
//! their heights; `count = 1` is the box layout itself.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{
    ColumnSizing, ColumnsLayout, Line, TextAlign, TextColumns, TextError, TextStyle, layout_box,
    layout_columns,
};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
const DPI: f64 = 96.0;
const WORDS: [&str; 9] = [
    "gafira", "text", "box", "column", "lorem", "ipsum", "dolor", "sit", "amet",
];

fn style(align: TextAlign) -> TextStyle {
    TextStyle {
        size_pt: 12.0,
        align,
        ..TextStyle::default()
    }
}

fn spec(count: u32, gap: f64, balance: bool) -> TextColumns {
    TextColumns {
        count,
        gap,
        balance,
    }
}

fn lorem(chars: usize) -> String {
    let mut text = String::new();
    let mut i = 0;
    while text.len() < chars {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(WORDS[i * 7 % WORDS.len()]);
        i += 1;
    }
    text.truncate(chars);
    text
}

fn run_with(
    text: &str,
    fonts: &[&[u8]],
    align: TextAlign,
    (width, height): (f64, f64),
    columns: &TextColumns,
    sizing: ColumnSizing,
) -> Result<ColumnsLayout, TextError> {
    layout_columns(
        text,
        fonts,
        &style(align),
        DPI,
        Size::new(width, height),
        columns,
        sizing,
    )
}

fn run(
    text: &str,
    width: f64,
    height: f64,
    columns: &TextColumns,
    sizing: ColumnSizing,
) -> ColumnsLayout {
    run_with(
        text,
        &[INTER],
        TextAlign::Left,
        (width, height),
        columns,
        sizing,
    )
    .expect("valid input")
}

fn lines_of(out: &ColumnsLayout) -> Vec<&Line> {
    out.columns.iter().flat_map(|c| &c.lines).collect()
}

fn line_height() -> f64 {
    let one = layout_box(
        "a",
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(100.0, 100.0),
    )
    .expect("valid input");
    f64::from(one.height)
}

/// The heights of the lines of the text in one column of `width`, as the box lays it out.
fn line_heights(text: &str, fonts: &[&[u8]], width: f64) -> Vec<f64> {
    layout_box(
        text,
        fonts,
        &style(TextAlign::Left),
        DPI,
        Size::new(width, 1e6),
    )
    .expect("valid input")
    .lines
    .iter()
    .map(|l| f64::from(l.height))
    .collect()
}

fn counts(out: &ColumnsLayout) -> Vec<usize> {
    out.columns.iter().map(|c| c.lines.len()).collect()
}

/// The byte ranges of the clusters in reading order cover the whole text, once each.
fn assert_reading_order_covers(text: &str, out: &ColumnsLayout) {
    let mut ranges: Vec<_> = lines_of(out)
        .iter()
        .flat_map(|l| l.clusters.iter().map(|c| c.range.clone()))
        .collect();
    ranges.sort_by_key(|r| r.start);
    let mut at = 0;
    for r in ranges {
        assert_eq!(r.start, at, "a cluster is missing or repeated at {at}");
        at = r.end;
    }
    assert_eq!(at, text.len(), "the text ends at {at}");
    let starts: Vec<usize> = lines_of(out)
        .iter()
        .map(|l| l.clusters.iter().map(|c| c.range.start).min().unwrap_or(0))
        .collect();
    assert!(
        starts.windows(2).all(|w| w[0] <= w[1]),
        "lines are in reading order"
    );
}

#[test]
fn fd_45_a_the_height_of_a_line_is_exposed_and_adds_up() {
    let text = "ab\n\u{627}\ncd cd cd\nx";
    let out = layout_box(
        text,
        &[INTER, ARABIC],
        &style(TextAlign::Left),
        DPI,
        Size::new(40.0, 500.0),
    )
    .expect("valid input");
    let heights: Vec<f64> = out.lines.iter().map(|l| f64::from(l.height)).collect();
    assert!(heights.iter().all(|h| *h > 0.0), "heights {heights:?}");
    assert!(
        (heights.iter().sum::<f64>() - f64::from(out.height)).abs() < 1e-3,
        "{heights:?} add up to {}",
        out.height
    );
    let same = layout_box(
        "ab\ncd\nef",
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(40.0, 500.0),
    )
    .expect("valid input");
    for pair in same.lines.windows(2) {
        assert!(
            (f64::from(pair[1].baseline - pair[0].baseline) - f64::from(pair[1].height)).abs()
                <= 0.01,
            "lines of one font are one height apart"
        );
    }
    assert!(
        heights.iter().any(|h| (h - heights[0]).abs() > 1.0),
        "the fixture mixes line heights: {heights:?}"
    );
}

#[test]
fn fd_45_a_three_columns_of_3000_characters_differ_by_at_most_one_line() {
    let text = lorem(3000);
    let lh = line_height();
    let mut residues = [false; 3];
    for step in 0..40 {
        let width = 480.0 + 4.0 * f64::from(step);
        let out = run(&text, width, 1e6, &spec(3, 10.0, true), ColumnSizing::Fixed);
        assert_eq!(out.columns.len(), 3);
        let heights: Vec<f64> = out.columns.iter().map(|c| f64::from(c.height)).collect();
        let max = heights.iter().copied().fold(f64::MIN, f64::max);
        let min = heights.iter().copied().fold(f64::MAX, f64::min);
        assert!(max - min <= lh + 0.01, "w={width}: heights {heights:?}");
        residues[counts(&out).iter().sum::<usize>() % 3] = true;
        assert_reading_order_covers(&text, &out);
        assert!(!out.overflow, "w={width}");
    }
    assert!(residues.iter().all(|r| *r), "all three residues of n mod 3");
}

/// Every split of `heights` into `k` runs of lines (empty runs allowed), as the lines per column.
fn all_splits(heights: &[f64], k: usize) -> Vec<Vec<usize>> {
    fn rec(k: usize, left: usize, acc: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if acc.len() == k - 1 {
            acc.push(left);
            out.push(acc.clone());
            acc.pop();
            return;
        }
        for take in 0..=left {
            acc.push(take);
            rec(k, left - take, acc, out);
            acc.pop();
        }
    }
    let mut out = Vec::new();
    rec(k, heights.len(), &mut Vec::new(), &mut out);
    out
}

fn column_heights(heights: &[f64], split: &[usize]) -> Vec<f64> {
    let mut start = 0;
    split
        .iter()
        .map(|n| {
            let h = heights[start..start + n].iter().sum::<f64>();
            start += n;
            h
        })
        .collect()
}

fn spread(cols: &[f64]) -> f64 {
    cols.iter().copied().fold(f64::MIN, f64::max) - cols.iter().copied().fold(f64::MAX, f64::min)
}

/// The split with the smallest difference between the tallest and the shortest column, each
/// column within `limit`; a tie goes to the one with the most lines in the earlier columns.
/// `None` when no split fits.
fn best_split(heights: &[f64], k: usize, limit: f64) -> Option<Vec<usize>> {
    let mut best: Option<(f64, Vec<usize>)> = None;
    for split in all_splits(heights, k) {
        let cols = column_heights(heights, &split);
        if cols.iter().any(|h| *h > limit) {
            continue;
        }
        let d = spread(&cols);
        let better = match &best {
            None => true,
            Some((bd, bs)) => d < bd - 1e-9 || ((d - bd).abs() <= 1e-9 && split > *bs),
        };
        if better {
            best = Some((d, split));
        }
    }
    best.map(|(_, s)| s)
}

/// The smallest height that holds the text in `k` columns.
fn least_height(heights: &[f64], k: usize) -> f64 {
    all_splits(heights, k)
        .iter()
        .map(|s| {
            column_heights(heights, s)
                .into_iter()
                .fold(f64::MIN, f64::max)
        })
        .fold(f64::MAX, f64::min)
}

#[test]
fn fd_45_a_balance_minimises_the_difference_of_heights_when_lines_differ() {
    let fonts: [&[u8]; 2] = [INTER, ARABIC];
    // Every sequence of 7 lines, and three of 9 for which the best split is taller than the least
    // box (it does not fit a box of that height), each line Latin (19.4 px) or Arabic (33.8 px),
    // in 2 to 4 columns:
    // balanced in a box of the least height that holds the text, balanced with a roomy box,
    // and with auto height. Each equals the best split found by trying them all.
    let mut binding = 0;
    let sequences = (0..128_u32)
        .map(|b| (7, b))
        .chain([(9, 43), (9, 51), (9, 408)]);
    for (n, bits) in sequences {
        let lines: Vec<&str> = (0..n)
            .map(|i| if bits >> i & 1 == 1 { "\u{627}" } else { "a" })
            .collect();
        let text = lines.join("\n");
        let heights = line_heights(&text, &fonts, 100.0);
        for k in 2..=4_u32 {
            let tight = least_height(&heights, k as usize);
            let roomy = tight * 2.0;
            let unbound = best_split(&heights, k as usize, f64::MAX).expect("a split fits");
            if column_heights(&heights, &unbound)
                .iter()
                .any(|h| *h > tight + 1e-9)
            {
                binding += 1;
            }
            for (sizing, height, limit) in [
                (ColumnSizing::AutoHeight, 0.0, f64::MAX),
                (ColumnSizing::Fixed, tight, tight),
                (ColumnSizing::Fixed, roomy, roomy),
            ] {
                let width = 100.0 * f64::from(k) + 10.0 * f64::from(k - 1);
                let out = run_with(
                    &text,
                    &fonts,
                    TextAlign::Left,
                    (width, height),
                    &spec(k, 10.0, true),
                    sizing,
                )
                .expect("valid input");
                let expected = best_split(&heights, k as usize, limit).expect("a split fits");
                assert_eq!(counts(&out), expected, "k={k} {text:?} {sizing:?} {height}");
                assert!(!out.overflow, "the text fits the box");
            }
        }
    }
    assert!(binding >= 3, "the box limits the best split: {binding}");
}

#[test]
fn fd_45_columns_are_equal_in_width_and_start_one_gap_apart() {
    let text = lorem(900);
    let (count, gap, width) = (3_u32, 20.0, 400.0);
    let cw = (width - 2.0 * gap) / 3.0;
    let out = run(
        &text,
        width,
        1e6,
        &spec(count, gap, false),
        ColumnSizing::Fixed,
    );
    let boxed = layout_box(
        &text,
        &[INTER],
        &style(TextAlign::Left),
        DPI,
        Size::new(cw, 1e6),
    )
    .expect("valid input");
    for (i, col) in out.columns.iter().enumerate() {
        let x = i as f64 * (cw + gap);
        assert!(
            (f64::from(col.x) - x).abs() <= 0.01,
            "column {i} at {}",
            col.x
        );
        assert!((f64::from(col.width) - cw).abs() <= 0.01);
        for line in &col.lines {
            let first = line.clusters.iter().map(|c| c.x).fold(f32::MAX, f32::min);
            assert!(
                (f64::from(first) - x).abs() <= 0.01,
                "a line of {i} starts at {first}"
            );
            let end = line
                .clusters
                .iter()
                .filter(|c| !text[c.range.clone()].trim().is_empty())
                .map(|c| f64::from(c.x + c.advance))
                .fold(f64::MIN, f64::max);
            assert!(end <= x + cw + 0.01, "a line of {i} ends at {end}");
        }
    }
    // One column fills everything in the roomy box: the wraps are the box layout at `cw`.
    let ranges = |lines: &[&Line]| -> Vec<std::ops::Range<usize>> {
        lines
            .iter()
            .flat_map(|l| l.clusters.iter().map(|c| c.range.clone()))
            .collect()
    };
    let flat_out: Vec<&Line> = lines_of(&out);
    let flat_box: Vec<&Line> = boxed.lines.iter().collect();
    assert_eq!(ranges(&flat_out), ranges(&flat_box));
    assert_eq!(flat_out.len(), flat_box.len());
}

#[test]
fn fd_45_a_text_split_by_height_fills_each_column_to_the_box() {
    let text = (0..14).map(|_| "ab").collect::<Vec<_>>().join("\n");
    let heights = line_heights(&text, &[INTER], 100.0);
    assert_eq!(heights.len(), 14);
    let five: f64 = heights[..5].iter().sum();
    // Three columns of five lines take 14 lines: 5, 5, 4. Equality fits.
    let out = run(
        &text,
        320.0,
        five,
        &spec(3, 10.0, false),
        ColumnSizing::Fixed,
    );
    assert_eq!(counts(&out), vec![5, 5, 4]);
    assert!(!out.overflow);
    assert_reading_order_covers(&text, &out);
    // A hair less holds four per column: the text does not fit and the last column keeps the rest.
    let out = run(
        &text,
        320.0,
        five - 1e-6,
        &spec(3, 10.0, false),
        ColumnSizing::Fixed,
    );
    assert_eq!(counts(&out), vec![4, 4, 6]);
    assert!(out.overflow, "the last column is taller than the box");
    assert_reading_order_covers(&text, &out);
    // Each column starts again at the top of the box.
    let baseline0 = out.columns[0].lines[0].baseline;
    for c in &out.columns {
        assert_eq!(c.lines[0].baseline, baseline0);
    }
}

#[test]
fn fd_45_a_line_taller_than_the_box_still_takes_a_column_and_flags_overflow() {
    let text = "ab\ncd\nef";
    let out = run(text, 220.0, 1.0, &spec(2, 10.0, false), ColumnSizing::Fixed);
    assert_eq!(counts(&out), vec![1, 2]);
    assert!(out.overflow);
    assert_reading_order_covers(text, &out);
}

#[test]
fn fd_45_c_overflow_shows_when_the_text_does_not_fit_and_not_before() {
    let text = (0..10).map(|_| "ab").collect::<Vec<_>>().join("\n");
    let heights = line_heights(&text, &[INTER], 100.0);
    let five: f64 = heights[..5].iter().sum();
    let fits = |height: f64, count: u32, balance: bool| {
        run(
            &text,
            220.0,
            height,
            &spec(count, 10.0, balance),
            ColumnSizing::Fixed,
        )
        .overflow
    };
    assert!(!fits(five, 2, false), "10 lines in 2 columns of 5");
    assert!(!fits(five, 2, true));
    assert!(fits(five - 1e-6, 2, false));
    assert!(fits(five - 1e-6, 2, true));
    let total: f64 = heights.iter().sum();
    assert!(!fits(total, 1, false), "one column that fits");
    assert!(fits(total - 1e-6, 1, false), "one column that does not");
    // A cluster wider than its column is an overflow of the text, also with room in height.
    let wide = "WWWW";
    let narrow = run(wide, 30.0, 1e6, &spec(2, 10.0, false), ColumnSizing::Fixed);
    assert!(narrow.overflow, "columns of 10 px");
    let roomy = run(wide, 300.0, 1e6, &spec(2, 10.0, false), ColumnSizing::Fixed);
    assert!(!roomy.overflow);
    // Auto height has no box to leave.
    let auto = run(
        &text,
        220.0,
        0.0,
        &spec(2, 10.0, false),
        ColumnSizing::AutoHeight,
    );
    assert!(!auto.overflow);
}

#[test]
fn fd_45_auto_height_balances_and_takes_its_height_from_the_columns() {
    let text = lorem(1200);
    let lh = line_height();
    for given in [0.0, 17.0, 1e6] {
        let auto = run(
            &text,
            300.0,
            given,
            &spec(3, 8.0, false),
            ColumnSizing::AutoHeight,
        );
        let balanced = run(&text, 300.0, 1e6, &spec(3, 8.0, true), ColumnSizing::Fixed);
        assert_eq!(auto, balanced, "the box height {given} is not used");
        let tallest = auto
            .columns
            .iter()
            .map(|c| f64::from(c.height))
            .fold(f64::MIN, f64::max);
        assert_eq!(f64::from(auto.height), tallest);
        let shortest = auto
            .columns
            .iter()
            .map(|c| f64::from(c.height))
            .fold(f64::MAX, f64::min);
        assert!(tallest - shortest <= lh + 0.01);
    }
}

#[test]
fn fd_45_auto_height_is_the_tallest_column_when_the_lines_differ_in_height() {
    // Line 2 is Arabic and taller than the Latin ones; the split gives the first column the
    // shorter share, so the box height is not the height of column 0.
    let text = "a\n\u{627}\u{644}\u{639}\na\na\n";
    let out = run_with(
        text,
        &[INTER, ARABIC],
        TextAlign::Left,
        (400.0, 0.0),
        &spec(2, 10.0, false),
        ColumnSizing::AutoHeight,
    )
    .expect("valid input");
    let tallest = out
        .columns
        .iter()
        .map(|c| c.height)
        .fold(f32::MIN, f32::max);
    assert!(
        out.columns[0].height < tallest,
        "the fixture needs a first column shorter than the tallest"
    );
    assert_eq!(out.height, tallest);
}

#[test]
fn fd_45_columns_work_with_justify() {
    let text = lorem(500);
    let (count, gap, width) = (2_u32, 10.0, 210.0);
    let cw = (width - gap) / 2.0;
    let out = run_with(
        &text,
        &[INTER],
        TextAlign::Justify,
        (width, 1e6),
        &spec(count, gap, true),
        ColumnSizing::Fixed,
    )
    .expect("valid input");
    let all = lines_of(&out);
    let mut stretched = 0;
    for (i, col) in out.columns.iter().enumerate() {
        for (j, line) in col.lines.iter().enumerate() {
            if std::ptr::eq(line, *all.last().expect("lines")) {
                continue;
            }
            let ink: Vec<_> = line
                .clusters
                .iter()
                .filter(|c| !text[c.range.clone()].trim().is_empty())
                .collect();
            let (first, last) = (
                ink.iter().map(|c| c.x).fold(f32::MAX, f32::min),
                ink.iter().map(|c| c.x).fold(f32::MIN, f32::max),
            );
            if !line
                .clusters
                .iter()
                .any(|c| &text[c.range.clone()] == " " && c.x > first && c.x < last)
            {
                continue;
            }
            let end = ink
                .iter()
                .map(|c| f64::from(c.x + c.advance))
                .fold(f64::MIN, f64::max);
            let edge = f64::from(col.x) + cw;
            assert!(
                (end - edge).abs() <= 0.01,
                "column {i} line {j} ends at {end}"
            );
            stretched += 1;
        }
    }
    assert!(stretched > 10);
}

#[test]
fn fd_45_d_the_numbers_are_checked() {
    let text = lorem(200);
    let call = |count: u32, gap: f64, width: f64| {
        run_with(
            &text,
            &[INTER],
            TextAlign::Left,
            (width, 500.0),
            &spec(count, gap, false),
            ColumnSizing::Fixed,
        )
        .err()
    };
    let range = |field| Some(TextError::OutOfRange { field });
    assert_eq!(call(0, 10.0, 400.0), range("columns.count"));
    assert_eq!(call(9, 10.0, 400.0), range("columns.count"));
    assert_eq!(call(u32::MAX, 10.0, 400.0), range("columns.count"));
    assert_eq!(call(2, -0.01, 400.0), range("columns.gap"));
    assert_eq!(call(2, 200.01, 1000.0), range("columns.gap"));
    assert_eq!(
        call(1, -0.01, 400.0),
        range("columns.gap"),
        "also with one column"
    );
    assert_eq!(call(1, 200.01, 400.0), range("columns.gap"));
    // The bounds are in.
    for (count, gap) in [(1, 0.0), (8, 0.0), (2, 200.0), (8, 200.0), (1, 200.0)] {
        assert_eq!(call(count, gap, 3000.0), None, "{count} columns, gap {gap}");
    }
    // A column of 1 px is in, a hair less is out.
    assert_eq!(call(3, 10.0, 23.0), None);
    assert_eq!(call(3, 10.0, 22.99), range("columns.width"));
    assert_eq!(call(2, 9.0, 10.0), range("columns.width"));
    assert_eq!(call(2, 0.0, 0.0), range("columns.width"));
    // One column has no gap to take from the width, so a box of size 0 is not an error.
    assert_eq!(call(1, 0.0, 0.0), None);
    // NaN and infinity come first, and the count is checked before the gap and the width.
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            call(0, bad, 400.0),
            Some(TextError::NonFinite {
                field: "columns.gap"
            })
        );
    }
    assert_eq!(call(0, -1.0, 1.0), range("columns.count"));
    assert_eq!(call(2, -1.0, 1.0), range("columns.gap"));
    // The font stack is checked after the columns.
    let no_font = layout_columns(
        "a",
        &[],
        &style(TextAlign::Left),
        DPI,
        Size::new(100.0, 100.0),
        &spec(0, 0.0, false),
        ColumnSizing::Fixed,
    );
    assert_eq!(no_font.err(), range("columns.count"));
    let no_font = layout_columns(
        "a",
        &[],
        &style(TextAlign::Left),
        DPI,
        Size::new(100.0, 100.0),
        &spec(2, 0.0, false),
        ColumnSizing::Fixed,
    );
    assert_eq!(no_font.err(), Some(TextError::NoFont));
}

#[test]
fn fd_45_e_one_column_is_the_box_layout_bit_for_bit_with_any_gap() {
    let texts = [
        "",
        "Hello world, a text that wraps in a box",
        "abc \u{627}\u{628}\u{62c} def \u{627}\u{628}\u{62c}",
        "first\nsecond paragraph\r\nthird",
    ];
    let fonts: [&[u8]; 2] = [INTER, ARABIC];
    for text in texts {
        for width in [0.0, 30.0, 300.0] {
            for align in [TextAlign::Left, TextAlign::Justify] {
                let plain = layout_box(text, &fonts, &style(align), DPI, Size::new(width, 20.0))
                    .expect("valid input");
                for gap in [0.0, 0.5, 37.0, 200.0] {
                    for balance in [false, true] {
                        for sizing in [ColumnSizing::Fixed, ColumnSizing::AutoHeight] {
                            let out = run_with(
                                text,
                                &fonts,
                                align,
                                (width, 20.0),
                                &spec(1, gap, balance),
                                sizing,
                            )
                            .expect("valid input");
                            assert_eq!(out.columns.len(), 1);
                            let col = &out.columns[0];
                            assert_eq!(
                                format!("{:?}", col.lines),
                                format!("{:?}", plain.lines),
                                "{text:?} w={width} gap={gap}"
                            );
                            assert_eq!(col.height.to_bits(), plain.height.to_bits());
                            assert_eq!(out.height.to_bits(), plain.height.to_bits());
                            assert_eq!(col.x.to_bits(), 0.0_f32.to_bits());
                            assert_eq!(f64::from(col.width), width);
                            if sizing == ColumnSizing::AutoHeight {
                                assert_eq!(out.overflow, plain.overflow);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn fd_45_a_with_fewer_lines_than_columns_the_last_columns_stay_empty() {
    // Balanced, two lines go one to each of three columns; filled, both go to the first.
    for (text, balance, expected) in [
        ("", true, vec![1, 0, 0]),
        ("", false, vec![1, 0, 0]),
        ("ab\ncd", true, vec![1, 1, 0]),
        ("ab\ncd", false, vec![2, 0, 0]),
    ] {
        let out = run(
            text,
            320.0,
            1e6,
            &spec(3, 10.0, balance),
            ColumnSizing::Fixed,
        );
        assert_eq!(out.columns.len(), 3);
        assert_eq!(counts(&out), expected, "{text:?} balance={balance}");
        let last = &out.columns[2];
        assert_eq!(last.height, 0.0);
        assert!((f64::from(last.x) - 2.0 * (100.0 + 10.0)).abs() <= 0.01);
        assert!(!out.overflow);
    }
}
