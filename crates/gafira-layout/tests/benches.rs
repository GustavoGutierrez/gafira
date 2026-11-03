//! ARQ-114: every figure of 02 §6.7 that a layout card cites has a benchmark with its name.

/// The benchmark file, read at build time (domain crates read no files at run time).
const BENCHES: &str = include_str!("../benches/layout.rs");

#[test]
fn arq_114_each_layout_figure_has_a_benchmark_named_after_its_row() {
    for name in [
        "bench_guides_generate", // FD-04, < 16 ms per change of value
        "bench_layout_rects",    // FD-05, < 1 ms per page
        "bench_snap_1000",       // FD-06, p99 < 2 ms with 1 000 nodes
        "bench_grid_lines",      // FD-07, < 1 ms
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
