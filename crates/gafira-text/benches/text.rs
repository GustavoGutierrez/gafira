//! Benchmarks of the layer-2 text figures (ARQ-114, 02 §6.7).
//!
//! One benchmark per row, named like the row. The budgets are checked by the nightly job on
//! the reference machine, not by `cargo test`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{
    ColumnSizing, FitBox, FitRun, TextAlign, TextColumns, TextStyle, fit_box, layout_box,
    layout_columns, layout_curved,
};

const DPI: f64 = 96.0;
const STACK: [&[u8]; 2] = [fonts::INTER.bytes, fonts::NOTO_SANS_ARABIC.bytes];
const WORDS: [&str; 9] = [
    "gafira", "text", "box", "column", "lorem", "ipsum", "dolor", "sit", "amet",
];

/// `chars` characters of Latin words, the same every time.
fn text(chars: usize) -> String {
    let mut text = String::with_capacity(chars + 8);
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

fn style() -> TextStyle {
    TextStyle {
        size_pt: 12.0,
        align: TextAlign::Left,
        ..TextStyle::default()
    }
}

/// FD-41: typing one character into a box of 5 000 characters lays the whole text out again.
fn bench_text_type_5000(c: &mut Criterion) {
    let mut typed = text(5000);
    typed.push('x');
    let style = style();
    c.bench_function("bench_text_type_5000", |b| {
        b.iter(|| {
            layout_box(
                black_box(&typed),
                black_box(&STACK),
                black_box(&style),
                DPI,
                Size::new(600.0, 4000.0),
            )
        });
    });
}

/// FD-44: the slider moves, so 100 characters are laid out and curved again.
fn bench_text_curve_100(c: &mut Criterion) {
    let text = text(100);
    let style = style();
    c.bench_function("bench_text_curve_100", |b| {
        b.iter(|| {
            layout_curved(
                black_box(&text),
                black_box(&STACK),
                black_box(&style),
                DPI,
                Size::new(600.0, 400.0),
                black_box(73.0),
            )
        });
    });
}

/// FD-45: 5 000 characters in 4 balanced columns.
fn bench_text_columns_5000(c: &mut Criterion) {
    let text = text(5000);
    let style = style();
    let columns = TextColumns {
        count: 4,
        gap: 16.0,
        balance: true,
    };
    c.bench_function("bench_text_columns_5000", |b| {
        b.iter(|| {
            layout_columns(
                black_box(&text),
                black_box(&STACK),
                black_box(&style),
                DPI,
                Size::new(900.0, 2000.0),
                black_box(&columns),
                ColumnSizing::Fixed,
            )
        });
    });
}

/// FD-46: the search over the default range for 500 characters.
fn bench_text_fit_500(c: &mut Criterion) {
    let text = text(500);
    let runs = [FitRun {
        text: &text,
        size_pt: 16.0,
    }];
    let style = style();
    let fit = FitBox::default();
    c.bench_function("bench_text_fit_500", |b| {
        b.iter(|| {
            fit_box(
                black_box(&runs),
                black_box(&STACK),
                black_box(&style),
                DPI,
                Size::new(400.0, 300.0),
                black_box(&fit),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_text_type_5000,
    bench_text_curve_100,
    bench_text_columns_5000,
    bench_text_fit_500
);
criterion_main!(benches);
