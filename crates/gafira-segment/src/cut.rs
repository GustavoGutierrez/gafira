//! GrabCut on the superpixel graph: two colour GMMs, a min-cut, up to five rounds.

use gafira_color::{Lab, LinearRgba};
use gafira_doc::contract::Cancelled;

use crate::flow::{Flow, INF};
use crate::gmm::Gmm;
use crate::meter::{CHUNK_ROWS, Meter};
use crate::slic::Superpixels;
use crate::work::WorkImage;

/// Weight of the smoothness term (02 §4.7, "gamma about 500").
const GAMMA: f64 = 500.0;
/// Rounds of fit and cut (02 §4.7).
const ROUNDS: u64 = 5;
/// Fixed-point scale of the capacities.
const SCALE: f64 = 16.0;

/// A superpixel: mean colour, size, centroid and whether it touches the border band.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Node {
    pub(crate) lab: [f64; 3],
    pub(crate) pixels: u32,
    pub(crate) centre: [f32; 2],
    pub(crate) in_band: bool,
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
    let mut in_band = vec![false; sp.count];
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
                    in_band[l] = true;
                }
            }
        }
    }
    Ok(sums
        .iter()
        .zip(&in_band)
        .map(|(s, &in_band)| {
            let n = s[0].max(1) as f64;
            let rgb = [s[3], s[4], s[5]].map(|v| decode(v as f64 / n));
            let lab = Lab::from_linear_srgb(LinearRgba::new(rgb[0], rgb[1], rgb[2], 1.0));
            Node {
                lab: [lab.l, lab.a, lab.b],
                pixels: s[0] as u32,
                centre: [s[1] as f32 / n as f32, s[2] as f32 / n as f32],
                in_band,
            }
        })
        .collect())
}

/// `(cost as foreground, cost as background)` of every node, from one GMM per
/// side fitted on the nodes already on that side. `None` if a side is empty.
pub(crate) fn unary_costs(nodes: &[Node], foreground: &[bool]) -> Option<Vec<(f64, f64)>> {
    let side = |want: bool| -> Option<Gmm> {
        let picked: Vec<&Node> = nodes
            .iter()
            .zip(foreground)
            .filter(|(_, f)| **f == want)
            .map(|(n, _)| n)
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

/// Runs GrabCut. Returns, per superpixel, whether it is foreground.
pub(crate) fn grabcut(
    nodes: &[Node],
    edges: &[(u32, u32)],
    pins: &[Pin],
    meter: &Meter<'_>,
    round_units: u64,
) -> Result<Vec<bool>, Cancelled> {
    let n = nodes.len();
    let apply = |labels: &mut [bool]| {
        for (l, p) in labels.iter_mut().zip(pins) {
            match p {
                Pin::Foreground => *l = true,
                Pin::Background => *l = false,
                Pin::Free => {}
            }
        }
    };
    let mut labels: Vec<bool> = nodes.iter().map(|v| !v.in_band).collect();
    apply(&mut labels);
    // Smoothness: strong between alike colours, weak across an edge.
    let d2: Vec<f64> = edges
        .iter()
        .map(|&(a, b)| {
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
        let Some(costs) = unary_costs(nodes, &labels) else {
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
        for (&(a, b), d) in edges.iter().zip(&d2) {
            let cap = fixed_point(GAMMA * (-beta * d).exp());
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
    edges: &[(u32, u32)],
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
    for &(a, b) in edges {
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

    fn chain() -> (Vec<Node>, Vec<(u32, u32)>) {
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
                in_band: i == 7 || i == 6,
            })
            .collect();
        (nodes, (0..7).map(|i| (i, i + 1)).collect())
    }

    // ARQ-130: a pinned superpixel keeps the side of its pin whatever the colours say.
    #[test]
    fn arq_130_pins_are_hard_constraints_of_the_cut() {
        use gafira_doc::contract::{CancelToken, NoProgress};
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

    use crate::slic::slic;
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
        let foreground: Vec<bool> = nodes.iter().map(|n| !n.in_band).collect();
        assert!(foreground.contains(&true) && foreground.contains(&false));
        let costs = unary_costs(&nodes, &foreground).unwrap();
        assert_eq!(
            [costs[left].0.to_bits(), costs[left].1.to_bits()],
            [costs[right].0.to_bits(), costs[right].1.to_bits()]
        );
    }
}
