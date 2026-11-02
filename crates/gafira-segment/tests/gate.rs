//! ARQ-132 (D-20, D-45): the quality gate. The 16 scenes of the fixed set at
//! 4000x3000 (Q-49: long side above 2048, so the reduction is exercised) must
//! each reach IoU 0.90 against their exact mask and be reliable (ARQ-131). The
//! 12 fresh scenes of another seed only report their count: a warning. FD-38 (a).

use std::io::Write;
use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_raster::{ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SegHints, SegZone, Segmentation, Segmenter};
use gafira_testkit::scenes::{
    FRESH_COUNT, FreshReport, Scene, fixed_scene, fresh_report, fresh_scene, iou, meets_gate,
};

const W: u32 = 4000;
const H: u32 = 3000;

fn seg() -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    ClassicSegmenter::new(Arc::new(pool))
}

fn segment(scene: &Scene) -> Segmentation {
    let view = ImageView::new(scene.width, scene.height, PixelData::Rgba8(&scene.rgba)).unwrap();
    let hints = SegHints {
        zone: SegZone::Foreground,
        ..SegHints::default()
    };
    seg()
        .segment(view, &hints, &NoProgress, &CancelToken::new())
        .unwrap()
}

macro_rules! gate_scene_tests {
    ($($name:ident: $i:expr),* $(,)?) => {$(
        // ARQ-132, FD-38 (a): this scene reaches IoU 0.90 (blocking); ARQ-131: it is reliable.
        #[test]
        fn $name() {
            let scene = fixed_scene($i, W, H);
            let out = segment(&scene);
            let value = iou(out.mask.data(), &scene.truth);
            eprintln!("gate scene {} {}: IoU {value:.4}", $i, scene.name);
            assert!(meets_gate(out.mask.data(), &scene.truth), "{}: IoU {value:.4}", scene.name);
            assert!(out.reliable, "{}: not reliable", scene.name);
        }
    )*};
}

gate_scene_tests! {
    arq_132_gate_scene_00: 0,
    arq_132_gate_scene_01: 1,
    arq_132_gate_scene_02: 2,
    arq_132_gate_scene_03: 3,
    arq_132_gate_scene_04: 4,
    arq_132_gate_scene_05: 5,
    arq_132_gate_scene_06: 6,
    arq_132_gate_scene_07: 7,
    arq_132_gate_scene_08: 8,
    arq_132_gate_scene_09: 9,
    arq_132_gate_scene_10: 10,
    arq_132_gate_scene_11: 11,
    arq_132_gate_scene_12: 12,
    arq_132_gate_scene_13: 13,
    arq_132_gate_scene_14: 14,
    arq_132_gate_scene_15: 15,
}

/// The warning on the fresh set (D-45): runs `mask_of` on each of the 12 scenes
/// of the other seed and reports how many reach IoU 0.90. It never fails the run
/// because of that count.
fn fresh_warning(width: u32, height: u32, mask_of: impl Fn(&Scene) -> Vec<u8>) -> FreshReport {
    let ious: Vec<f64> = (0..FRESH_COUNT)
        .map(|i| {
            let scene = fresh_scene(i, width, height);
            let value = iou(&mask_of(&scene), &scene.truth);
            eprintln!("fresh scene {i} {}: IoU {value:.4}", scene.name);
            value
        })
        .collect();
    let report = fresh_report(&ious);
    // Straight to stderr: `eprintln!` is captured for a passing test, and the
    // count has to reach the log of a plain `cargo test` (ARQ-132).
    let line = format!(
        "WARNING: fresh scenes at IoU >= 0.90: {}/{} (a warning, not a gate)\n",
        report.passed, report.total
    );
    std::io::stderr()
        .write_all(line.as_bytes())
        .expect("stderr takes the count");
    report
}

// ARQ-132: the fresh scenes are a warning. A segmenter that fails on all of them
// reports 0/12 and the run does not fail.
// Mutant: the warning asserts on its count (made blocking).
#[test]
fn arq_132_a_segmenter_failing_every_fresh_scene_reports_0_of_12() {
    let report = fresh_warning(160, 120, |s| vec![0; s.truth.len()]);
    assert_eq!((report.passed, report.total), (0, 12));
}

// ARQ-132: the real run over the fresh set, at 4000x3000. Whatever the count, it
// passes: the count goes to the output and to the notes of the delivery.
#[test]
fn arq_132_fresh_scenes_report_their_count_and_do_not_block() {
    let report = fresh_warning(W, H, |s| segment(s).mask.data().to_vec());
    assert_eq!(report.total, FRESH_COUNT);
}
