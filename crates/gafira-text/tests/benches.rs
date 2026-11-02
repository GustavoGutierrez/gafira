//! ARQ-114: every figure of 02 §6.7 that a text card cites has a benchmark with its name.

/// The benchmark file, read at build time (domain crates read no files at run time).
const BENCHES: &str = include_str!("../benches/text.rs");

#[test]
fn arq_114_each_text_figure_has_a_benchmark_named_after_its_row() {
    for name in [
        "bench_text_type_5000",    // FD-41, p99 < 16 ms to type a character in 5 000
        "bench_text_curve_100",    // FD-44, p99 < 16 ms with 100 characters
        "bench_text_columns_5000", // FD-45, p99 < 16 ms for 5 000 characters in 4 columns
        "bench_text_fit_500",      // FD-46, p99 < 16 ms with 500 characters
    ] {
        let id = format!("\"{name}\"");
        assert_eq!(
            BENCHES.matches(&id).count(),
            1,
            "{name} must be registered once with criterion"
        );
    }
    assert!(BENCHES.contains("criterion_main!"));
}

#[test]
fn arq_114_each_benchmark_function_is_in_the_criterion_group() {
    let start = BENCHES
        .find("criterion_group!(")
        .expect("the benchmarks are grouped");
    let group = &BENCHES[start..start + BENCHES[start..].find(");").expect("the group closes")];
    for name in [
        "bench_text_type_5000",
        "bench_text_curve_100",
        "bench_text_columns_5000",
        "bench_text_fit_500",
    ] {
        assert!(
            group.contains(name),
            "{name} is defined but criterion_group! does not run it"
        );
    }
}
