//! S84-02: the quality of a photo.

use gafira_raster::{Quality, assess, sharpness};
use proptest::prelude::*;

/// A picture with edges of every size: leaves of noise over a gradient.
fn scene(w: usize, h: usize) -> Vec<f32> {
    let hash = |x: usize, y: usize| {
        let n = (x as u32)
            .wrapping_mul(374_761_393)
            .wrapping_add((y as u32).wrapping_mul(668_265_263));
        let n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
        ((n ^ (n >> 16)) & 0xFF) as f32
    };
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let leaf = hash(x / 9, y / 7) * 0.5 + hash(x / 40, y / 33) * 0.3 + hash(x, y) * 0.06;
            leaf + (x + y) as f32 * 40.0 / (w + h) as f32
        })
        .collect()
}

fn blurred(luma: &[f32], w: usize, radius: usize) -> Vec<f32> {
    let h = luma.len() / w;
    let pass = |src: &[f32], along: bool| -> Vec<f32> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let (mut sum, mut n) = (0.0, 0.0);
                for d in 0..=2 * radius {
                    let d = d as isize - radius as isize;
                    let (x, y) = if along {
                        (x as isize + d, y as isize)
                    } else {
                        (x as isize, y as isize + d)
                    };
                    if x >= 0 && y >= 0 && x < w as isize && y < h as isize {
                        sum += src[y as usize * w + x as usize];
                        n += 1.0;
                    }
                }
                sum / n
            })
            .collect()
    };
    let mut out = luma.to_vec();
    for _ in 0..2 {
        out = pass(&pass(&out, true), false);
    }
    out
}

/// Each block of eight with little more than its mean.
fn blocky(luma: &[f32], w: usize) -> Vec<f32> {
    let h = luma.len() / w;
    let mut out = luma.to_vec();
    for by in (0..h).step_by(8) {
        for bx in (0..w).step_by(8) {
            let cells: Vec<usize> = (by..(by + 8).min(h))
                .flat_map(|y| (bx..(bx + 8).min(w)).map(move |x| y * w + x))
                .collect();
            let mean = cells.iter().map(|i| luma[*i]).sum::<f32>() / cells.len() as f32;
            for i in cells {
                out[i] = mean + (luma[i] - mean) * 0.15;
            }
        }
    }
    out
}

fn rgba(luma: &[f32]) -> Vec<u8> {
    luma.iter()
        .flat_map(|v| {
            let v = v.clamp(0.0, 255.0) as u8;
            [v, v, v, 255]
        })
        .collect()
}

#[test]
fn s84_02_good_small_blurry_and_blocky() {
    let (w, h) = (1200, 900);
    let sharp = scene(w, h);
    let of = |luma: &[f32]| assess(w as u32, h as u32, &rgba(luma));
    let soft = blurred(&sharp, w, 1);
    let smeared = blurred(&sharp, w, 11);
    for (name, luma) in [("sharp", &sharp), ("soft", &soft), ("smeared", &smeared)] {
        println!("{name}: {:?}", sharpness(w as u32, h as u32, &rgba(luma)));
    }
    assert_eq!(of(&sharp), Quality::Good);
    assert_eq!(of(&soft), Quality::Good, "a little soft is fine");
    assert_eq!(of(&smeared), Quality::Blurry);
    assert_eq!(of(&blocky(&sharp, w)), Quality::Blocky);
    let small = scene(400, 300);
    assert_eq!(assess(400, 300, &rgba(&small)), Quality::Small);
}

// A flat picture has no edges to judge: there is nothing to make better.
#[test]
fn s84_02_flat_and_wrong_input() {
    assert_eq!(assess(1000, 1000, &vec![200; 4_000_000]), Quality::Good);
    assert_eq!(
        assess(1000, 1000, &[1, 2, 3]),
        Quality::Good,
        "not its bytes"
    );
    assert_eq!(assess(0, 0, &[]), Quality::Good);
}

// A large photo is judged at the size a model sees it.
#[test]
fn s84_02_large_images() {
    let sharp = scene(3000, 2200);
    assert_eq!(assess(3000, 2200, &rgba(&sharp)), Quality::Good);
}

proptest! {
    #[test]
    fn s84_02_any_image(w in 1_u32..40, h in 1_u32..40, bytes in prop::collection::vec(any::<u8>(), 6400)) {
        let _ = assess(w, h, &bytes[..(w * h * 4) as usize]);
    }
}
