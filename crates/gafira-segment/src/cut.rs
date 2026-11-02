//! GrabCut on the superpixel graph: two colour GMMs, a min-cut, up to five rounds.

use gafira_color::{Lab, LinearRgba};
use gafira_doc::contract::Cancelled;

use crate::flow::{Flow, INF};
use crate::gmm::Gmm;
use crate::meter::{CHUNK_ROWS, Meter};
use crate::slic::Superpixels;
use crate::work::WorkImage;

/// Weight of the smoothness term (02 §4.7, "gamma about 500").
pub(crate) const GAMMA: f64 = 500.0;
/// Rounds of fit and cut (02 §4.7).
const ROUNDS: u64 = 5;
/// Band colours closer than this, in ΔE76, are the same stretch of background.
const BAND_LINK: f64 = 6.0;
/// A band superpixel with fewer neighbours in colour is an edge mix, not a stretch.
const BAND_CORE: usize = 4;
/// Fixed-point scale of the capacities.
const SCALE: f64 = 16.0;

/// A superpixel: mean colour, size, centroid and its pixels in the border band.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Node {
    pub(crate) lab: [f64; 3],
    pub(crate) pixels: u32,
    pub(crate) centre: [f32; 2],
    pub(crate) band_pixels: u32,
}

impl Node {
    /// At least half of its pixels are in the band, `n` of `2n` included (ARQ-133).
    pub(crate) fn held(&self) -> bool {
        self.band_pixels > 0 && 2 * u64::from(self.band_pixels) >= u64::from(self.pixels)
    }
}

/// What the person pinned on a superpixel with seed strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pin {
    Free,
    Foreground,
    Background,
}

/// Planned work units of the graph stages, for the progress model.
pub(crate) fn planned(w: usize, h: usize) -> u64 {
    // Statistics and adjacency read every pixel once each; the rounds are light.
    (w * h) as u64 * 2 + ROUNDS * round_units(w, h)
}

/// Units one round of fit and cut books.
pub(crate) fn round_units(w: usize, h: usize) -> u64 {
    ((w * h) as u64 / 16).max(1)
}

fn decode(v: f64) -> f32 {
    let c = v / 255.0;
    (if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }) as f32
}

/// One node per superpixel, from a serial pass in chunks of rows.
pub(crate) fn nodes(
    img: &WorkImage,
    sp: &Superpixels,
    meter: &Meter<'_>,
) -> Result<Vec<Node>, Cancelled> {
    let (w, h) = (img.width, img.height);
    let band = (w.min(h) / 64).max(1);
    let mut sums = vec![[0u64; 6]; sp.count];
    let mut in_band = vec![0u32; sp.count];
    for y0 in (0..h).step_by(CHUNK_ROWS) {
        let y1 = (y0 + CHUNK_ROWS).min(h);
        meter.step(((y1 - y0) * w) as u64)?;
        for y in y0..y1 {
            for x in 0..w {
                let i = y * w + x;
                let l = sp.labels[i] as usize;
                let s = &mut sums[l];
                let [r, g, b] = img.rgb[i];
                s[0] += 1;
                s[1] += x as u64;
                s[2] += y as u64;
                s[3] += u64::from(r);
                s[4] += u64::from(g);
                s[5] += u64::from(b);
                if x < band || y < band || x + band >= w || y + band >= h {
                    in_band[l] += 1;
                }
            }
        }
    }
    Ok(sums
        .iter()
        .zip(&in_band)
        .map(|(s, &band_pixels)| {
            let n = s[0].max(1) as f64;
            let rgb = [s[3], s[4], s[5]].map(|v| decode(v as f64 / n));
            let lab = Lab::from_linear_srgb(LinearRgba::new(rgb[0], rgb[1], rgb[2], 1.0));
            Node {
                lab: [lab.l, lab.a, lab.b],
                pixels: s[0] as u32,
                centre: [s[1] as f32 / n as f32, s[2] as f32 / n as f32],
                band_pixels,
            }
        })
        .collect())
}

/// Superpixels of the band whose colour does not train the background model: a
/// subject touching the border puts its colours in the band, and left in the
/// model they would take the rest of the subject with them. Band colours are
/// grouped by nearness, so a gradient stays one group; the largest group is the
/// background, and a group under half of the band is kept out, as are isolated
/// colours, which are edge mixes. The pins do not change (ARQ-133).
fn strays(nodes: &[Node], pins: &[Pin]) -> Vec<bool> {
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let band: Vec<usize> = (0..nodes.len())
        .filter(|&i| nodes[i].band_pixels > 0 && pins[i] == Pin::Free)
        .collect();
    let near = |a: usize, b: usize| {
        let (x, y) = (nodes[band[a]].lab, nodes[band[b]].lab);
        (0..3).map(|i| (x[i] - y[i]).powi(2)).sum::<f64>() <= BAND_LINK * BAND_LINK
    };
    let core: Vec<bool> = (0..band.len())
        .map(|a| (0..band.len()).filter(|&b| b != a && near(a, b)).count() >= BAND_CORE)
        .collect();
    let mut parent: Vec<usize> = (0..band.len()).collect();
    for a in 0..band.len() {
        for b in 0..a {
            if core[a] && core[b] && near(a, b) {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
    }
    let mut size = vec![0u64; band.len()];
    let mut total = 0;
    for a in 0..band.len() {
        let pixels = u64::from(nodes[band[a]].pixels);
        total += pixels;
        if core[a] {
            size[find(&mut parent, a)] += pixels;
        }
    }
    let mut out = vec![false; nodes.len()];
    let top = size.iter().copied().max().unwrap_or(0);
    if top == 0 {
        return out;
    }
    for a in 0..band.len() {
        let group = size[find(&mut parent, a)];
        out[band[a]] = !core[a] || (group < top && 2 * group < total);
    }
    out
}

/// `(cost as foreground, cost as background)` of every node, from one GMM per
/// side fitted on the nodes already on that side, less those in `left_out`.
/// `None` if a side is empty.
pub(crate) fn unary_costs(
    nodes: &[Node],
    foreground: &[bool],
    left_out: &[bool],
) -> Option<Vec<(f64, f64)>> {
    let side = |want: bool| -> Option<Gmm> {
        let picked: Vec<&Node> = nodes
            .iter()
            .zip(foreground)
            .zip(left_out)
            .filter(|((_, f), out)| **f == want && !**out)
            .map(|((n, _), _)| n)
            .collect();
        let colours: Vec<[f64; 3]> = picked.iter().map(|n| n.lab).collect();
        let weights: Vec<f64> = picked.iter().map(|n| f64::from(n.pixels)).collect();
        Gmm::fit(&colours, &weights)
    };
    let (fg, bg) = (side(true)?, side(false)?);
    Some(
        nodes
            .iter()
            .map(|n| (fg.cost(n.lab).max(0.0), bg.cost(n.lab).max(0.0)))
            .collect(),
    )
}

fn fixed_point(v: f64) -> i64 {
    (v * SCALE).round() as i64
}

/// Capacity of the edge between two superpixels, from the squared colour
/// distance and the length of their shared border (ARQ-133).
fn edge_capacity(d2: f64, beta: f64, length: u32) -> i64 {
    fixed_point(GAMMA * f64::from(length) * (-beta * d2).exp())
}

/// Runs GrabCut. Returns, per superpixel, whether it is foreground.
pub(crate) fn grabcut(
    nodes: &[Node],
    edges: &[(u32, u32, u32)],
    pins: &[Pin],
    meter: &Meter<'_>,
    round_units: u64,
) -> Result<Vec<bool>, Cancelled> {
    let n = nodes.len();
    let left_out = strays(nodes, pins);
    // The seeds go first; a free superpixel of the band is pinned as background.
    let pins: Vec<Pin> = pins
        .iter()
        .zip(nodes)
        .map(|(pin, node)| match pin {
            Pin::Free if node.held() => Pin::Background,
            other => *other,
        })
        .collect();
    let pins = pins.as_slice();
    let apply = |labels: &mut [bool]| {
        for (l, p) in labels.iter_mut().zip(pins) {
            match p {
                Pin::Foreground => *l = true,
                Pin::Background => *l = false,
                Pin::Free => {}
            }
        }
    };
    let mut labels: Vec<bool> = nodes.iter().map(|v| v.band_pixels == 0).collect();
    apply(&mut labels);
    // Smoothness: strong between alike colours, weak across an edge.
    let d2: Vec<f64> = edges
        .iter()
        .map(|&(a, b, _)| {
            let (x, y) = (nodes[a as usize].lab, nodes[b as usize].lab);
            (0..3).map(|i| (x[i] - y[i]).powi(2)).sum()
        })
        .collect();
    let mean = d2.iter().sum::<f64>() / d2.len().max(1) as f64;
    let beta = if mean > 0.0 { 0.5 / mean } else { 0.0 };
    let mut spent = 0;
    for _ in 0..ROUNDS {
        meter.step(round_units)?;
        spent += 1;
        let Some(costs) = unary_costs(nodes, &labels, &left_out) else {
            break;
        };
        let (s, t) = (n, n + 1);
        let mut flow = Flow::new(n + 2);
        for (i, ((fg, bg), node)) in costs.iter().zip(nodes).enumerate() {
            let size = f64::from(node.pixels);
            let (to_s, to_t) = match pins[i] {
                Pin::Foreground => (INF, 0),
                Pin::Background => (0, INF),
                Pin::Free => (fixed_point(bg * size), fixed_point(fg * size)),
            };
            flow.add(s, i, to_s, 0);
            flow.add(i, t, to_t, 0);
        }
        for (&(a, b, length), d) in edges.iter().zip(&d2) {
            let cap = edge_capacity(*d, beta, length);
            flow.add(a as usize, b as usize, cap, cap);
        }
        let side = flow.source_side(s, t, meter)?;
        let mut next: Vec<bool> = side[..n].to_vec();
        apply(&mut next);
        if next == labels {
            break;
        }
        labels = next;
    }
    // Settle the units of the rounds that were not needed.
    if spent < ROUNDS {
        meter.step(round_units * (ROUNDS - spent))?;
    }
    Ok(labels)
}

/// Keeps only the foreground region nearest the centre of the `w` x `h` image
/// (the central priority of «Elemento», D-23), plus any region holding a
/// foreground seed. Regions are connected sets of foreground superpixels.
pub(crate) fn central(
    nodes: &[Node],
    edges: &[(u32, u32, u32)],
    foreground: &[bool],
    pins: &[Pin],
    (w, h): (usize, usize),
) -> Vec<bool> {
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut parent: Vec<usize> = (0..nodes.len()).collect();
    for &(a, b, _) in edges {
        let (a, b) = (a as usize, b as usize);
        if foreground[a] && foreground[b] {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            parent[ra.max(rb)] = ra.min(rb);
        }
    }
    let middle = [w as f32 / 2.0, h as f32 / 2.0];
    let nearest = (0..nodes.len())
        .filter(|&i| foreground[i])
        .min_by(|&a, &b| {
            let d = |i: usize| {
                (nodes[i].centre[0] - middle[0]).powi(2) + (nodes[i].centre[1] - middle[1]).powi(2)
            };
            d(a).total_cmp(&d(b)).then(a.cmp(&b))
        });
    let mut keep = vec![false; nodes.len()];
    if let Some(i) = nearest {
        let root = find(&mut parent, i);
        keep[root] = true;
    }
    for (i, pin) in pins.iter().enumerate() {
        if foreground[i] && *pin == Pin::Foreground {
            let root = find(&mut parent, i);
            keep[root] = true;
        }
    }
    (0..nodes.len())
        .map(|i| foreground[i] && keep[find(&mut parent, i)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> (Vec<Node>, Vec<(u32, u32, u32)>) {
        // Four red nodes then four grey ones, the first red one in the border band.
        let colour = |i: usize| {
            if i < 4 {
                [45.0, 60.0, 40.0]
            } else {
                [55.0, 0.0, 0.0]
            }
        };
        let nodes = (0..8)
            .map(|i| Node {
                lab: colour(i),
                pixels: 100,
                centre: [i as f32, 0.0],
                band_pixels: u32::from(i >= 6),
            })
            .collect();
        (nodes, (0..7).map(|i| (i, i + 1, 1)).collect())
    }

    // ARQ-130: a pinned superpixel keeps the side of its pin whatever the colours say.
    #[test]
    fn arq_130_pins_are_hard_constraints_of_the_cut() {
        let (nodes, edges) = chain();
        let (progress, cancel) = (NoProgress, CancelToken::new());
        let meter = Meter::new(&progress, &cancel, 1 << 30);
        let free = vec![Pin::Free; 8];
        let plain = grabcut(&nodes, &edges, &free, &meter, 1).unwrap();
        assert_eq!(
            plain,
            [true, true, true, true, false, false, false, false],
            "premise"
        );
        let mut pins = free;
        pins[1] = Pin::Background;
        pins[5] = Pin::Foreground;
        let pinned = grabcut(&nodes, &edges, &pins, &meter, 1).unwrap();
        assert!(!pinned[1], "red node pinned to background");
        assert!(pinned[5], "grey node pinned to foreground");
    }

    fn node(lab: [f64; 3], pixels: u32, band_pixels: u32) -> Node {
        Node {
            lab,
            pixels,
            centre: [0.0, 0.0],
            band_pixels,
        }
    }

    fn cut(nodes: &[Node], edges: &[(u32, u32, u32)], pins: &[Pin]) -> Vec<bool> {
        let (progress, cancel) = (NoProgress, CancelToken::new());
        let meter = Meter::new(&progress, &cancel, 1 << 30);
        grabcut(nodes, edges, pins, &meter, 1).unwrap()
    }

    const RED: [f64; 3] = [45.0, 60.0, 40.0];
    const GREY: [f64; 3] = [55.0, 0.0, 0.0];

    // ARQ-133: of two pairs with the same colours, the pair with twice the shared
    // border has twice the capacity, but for the rounding of the capacity.
    // Mutant: border weight off.
    #[test]
    fn arq_133_twice_the_border_is_twice_the_capacity() {
        for (d2, length) in [(0.0, 7), (10.0, 30), (200.0, 1), (5.0, 1000)] {
            let (one, two) = (
                edge_capacity(d2, 0.01, length),
                edge_capacity(d2, 0.01, 2 * length),
            );
            assert!((two - 2 * one).abs() <= 1, "{d2} {length}: {one} {two}");
        }
    }

    // ARQ-133: the cut follows the border length. A free superpixel between a
    // pinned foreground and a pinned background goes with the side it shares
    // more border with.
    // Mutant: border weight off.
    #[test]
    fn arq_133_a_longer_border_pulls_the_cut() {
        let mid = [50.0, 30.0, 20.0];
        let nodes = [node(RED, 1, 0), node(mid, 1, 0), node(GREY, 1, 0)];
        let pins = [Pin::Foreground, Pin::Free, Pin::Background];
        assert!(
            cut(&nodes, &[(0, 1, 90), (1, 2, 3)], &pins)[1],
            "long on the foreground"
        );
        assert!(
            !cut(&nodes, &[(0, 1, 3), (1, 2, 90)], &pins)[1],
            "long on the background"
        );
    }

    // ARQ-133: a superpixel of 2n pixels with n in the band is pinned as
    // background; with n - 1 it is not.
    // Mutants: band count `>` for `>=`; any pixel in the band pins.
    #[test]
    fn arq_133_half_of_the_pixels_in_the_band_pins() {
        for (pixels, band, held) in [
            (200, 100, true),
            (200, 99, false),
            (201, 100, false),
            (1, 1, true),
            (5, 0, false),
        ] {
            assert_eq!(node(RED, pixels, band).held(), held, "{band} of {pixels}");
        }
        // The count comes from the pixels of the band, 1 px wide on 64 x 64.
        let width = 64;
        let img = WorkImage {
            width,
            height: width,
            rgb: vec![[10, 20, 30]; width * width],
        };
        let mut labels = vec![0u32; width * width];
        // Label 1: 10 pixels of row 0 and 10 of row 1.
        for x in 10..20 {
            labels[x] = 1;
            labels[width + x] = 1;
        }
        // Label 2: the same, but one of the pixels of row 0 moved to row 2.
        for x in 30..40 {
            labels[x] = 2;
            labels[width + x] = 2;
        }
        labels[30] = 0;
        labels[2 * width + 30] = 2;
        let sp = Superpixels { labels, count: 3 };
        let (progress, cancel) = (NoProgress, CancelToken::new());
        let meter = Meter::new(&progress, &cancel, 1 << 30);
        let nodes = nodes(&img, &sp, &meter).unwrap();
        assert_eq!((nodes[1].pixels, nodes[1].band_pixels), (20, 10));
        assert_eq!((nodes[2].pixels, nodes[2].band_pixels), (20, 9));
        assert!(nodes[1].held() && !nodes[2].held());
    }

    // ARQ-133: with no seeds, no pinned superpixel ends in the foreground, whatever
    // its colour; a seed goes first and keeps it there.
    // Mutants: band pin off; band pin above the seeds.
    #[test]
    fn arq_133_pinned_nodes_stay_background_unless_seeded() {
        // Red nodes in the middle and on the band, grey around them.
        let mut nodes: Vec<Node> = (0..6)
            .map(|i| node(if i < 3 { RED } else { GREY }, 100, 0))
            .collect();
        nodes.push(node(RED, 100, 100));
        nodes.extend((0..6).map(|_| node(GREY, 100, 100)));
        let edges: Vec<(u32, u32, u32)> = (0..12).map(|i| (i, i + 1, 1)).collect();
        let mut pins = vec![Pin::Free; nodes.len()];
        let plain = cut(&nodes, &edges, &pins);
        assert!(plain[0], "premise: the free red node is foreground");
        assert!(!plain[6], "pinned red node");
        pins[6] = Pin::Foreground;
        assert!(cut(&nodes, &edges, &pins)[6], "a seed goes first");
    }

    // ARQ-133: if every superpixel is pinned, nothing is foreground, and nothing fails.
    #[test]
    fn arq_133_all_pinned_gives_no_foreground() {
        let nodes = [node(RED, 4, 4), node(GREY, 4, 2)];
        let got = cut(&nodes, &[(0, 1, 2)], &[Pin::Free, Pin::Free]);
        assert_eq!(got, [false, false]);
    }

    // ARQ-133: the two tuned values stay in their ranges, ends included.
    #[test]
    fn arq_133_parameters_are_in_their_ranges() {
        assert!((400.0..=600.0).contains(&GAMMA), "gamma {GAMMA}");
        let reg = crate::gmm::REGULARISATION;
        assert!((20.0..=30.0).contains(&reg), "regularisation {reg}");
    }

    use crate::slic::{Superpixels, slic};
    use gafira_doc::contract::{CancelToken, NoProgress};

    /// Colour X on the left and right thirds, colour Y in the middle.
    fn stripes() -> WorkImage {
        let (width, height) = (60, 24);
        let rgb = (0..width * height)
            .map(|i| {
                let x = i % width;
                if (20..40).contains(&x) {
                    [30, 60, 200]
                } else {
                    [210, 190, 40]
                }
            })
            .collect();
        WorkImage { width, height, rgb }
    }

    // ARQ-130: with no hints, two superpixels of the same mean colour at
    // different positions get, bit for bit, the same cost against each GMM.
    #[test]
    fn arq_130_same_colour_superpixels_cost_the_same_wherever_they_are() {
        let img = stripes();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let (progress, cancel) = (NoProgress, CancelToken::new());
        let meter = Meter::new(&progress, &cancel, 1 << 30);
        let sp = slic(&img, &pool, &meter).unwrap();
        let nodes = nodes(&img, &sp, &meter).unwrap();
        let left = sp.labels[12 * 60 + 2] as usize;
        let right = sp.labels[12 * 60 + 57] as usize;
        assert_ne!(left, right, "two different superpixels");
        assert_eq!(
            nodes[left].lab.map(f64::to_bits),
            nodes[right].lab.map(f64::to_bits)
        );
        let foreground: Vec<bool> = nodes.iter().map(|n| n.band_pixels == 0).collect();
        assert!(foreground.contains(&true) && foreground.contains(&false));
        let costs = unary_costs(&nodes, &foreground, &vec![false; nodes.len()]).unwrap();
        assert_eq!(
            [costs[left].0.to_bits(), costs[left].1.to_bits()],
            [costs[right].0.to_bits(), costs[right].1.to_bits()]
        );
    }
}
