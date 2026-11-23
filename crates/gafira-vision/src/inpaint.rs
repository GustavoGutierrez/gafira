//! Erasing a part of an image and making up what is behind it (spec 086).
//!
//! The model sees squares of one size. Each piece of the mask is worked on in
//! squares around it, as they are in the image when the piece is thin, such
//! as letters, and smaller when it is wide, so that there is always something
//! around to make it up from. A square sees what the ones before it filled.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::vision::{Inpainter, Pixels, VisionError};
use gafira_raster::AlphaMask;
use image::imageops::{self, FilterType};
use image::{GrayImage, Luma, Rgb, RgbImage};

use crate::catalog::Filling;
use crate::cutout::opaque;
use crate::runtime::Session;

/// The widest a hole is for the model, as a share of its side.
const WIDEST: f32 = 0.4;

/// The share of a square, all around, that is only there to be seen.
const AROUND: f32 = 0.18;

pub(crate) struct Filler {
    pub session: Arc<dyn Session>,
    pub side: usize,
    pub kind: Filling,
}

/// Pixels from the first, up to the last not included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Area {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

impl Area {
    fn size(self) -> (u32, u32) {
        (self.x1 - self.x0, self.y1 - self.y0)
    }

    /// This and `by` more all around, inside `w` × `h`.
    fn grown(self, by: u32, (w, h): (u32, u32)) -> Self {
        Self {
            x0: self.x0.saturating_sub(by),
            y0: self.y0.saturating_sub(by),
            x1: self.x1.saturating_add(by).min(w),
            y1: self.y1.saturating_add(by).min(h),
        }
    }

    fn meets(self, other: Self) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    fn with(self, other: Self) -> Self {
        Self {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }
}

fn cancelled(cancel: &CancelToken) -> Result<(), VisionError> {
    cancel.check().map_err(|_| VisionError::Cancelled)
}

/// How far each pixel that is not `out` is from one that is, in steps of a
/// third of a pixel (chamfer 3-4).
fn distances(out: &[bool], (w, h): (usize, usize)) -> Vec<u32> {
    let mut d: Vec<u32> = out
        .iter()
        .map(|out| if *out { 0 } else { u32::MAX / 2 })
        .collect();
    let near = |d: &[u32], x: usize, y: usize, dx: i64, dy: i64, cost: u32| {
        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
        let inside = nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64;
        // Inside: the cast is exact.
        inside.then(|| d[ny as usize * w + nx as usize].saturating_add(cost))
    };
    let pass = |d: &mut Vec<u32>, x: usize, y: usize, sign: i64| {
        let around = [(-1, 0, 3), (0, -1, 3), (-1, -1, 4), (1, -1, 4)];
        let least = around
            .iter()
            .filter_map(|(dx, dy, cost)| near(d, x, y, dx * sign, dy * sign, *cost))
            .min();
        if let Some(least) = least {
            let at = y * w + x;
            d[at] = d[at].min(least);
        }
    };
    for y in 0..h {
        for x in 0..w {
            pass(&mut d, x, y, 1);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            pass(&mut d, x, y, -1);
        }
    }
    d
}

/// The boxes of the pieces of `hole`, the ones that see each other together.
fn areas(hole: &[bool], (w, h): (u32, u32), reach: u32) -> Vec<Area> {
    let mut seen = vec![false; hole.len()];
    let mut out: Vec<Area> = Vec::new();
    let (wide, tall) = (w as usize, h as usize);
    for start in 0..hole.len() {
        if !hole[start] || seen[start] {
            continue;
        }
        let mut area: Option<Area> = None;
        let mut next = vec![start];
        seen[start] = true;
        while let Some(at) = next.pop() {
            let (x, y) = (at % wide, at / wide);
            // Within an image: the casts are exact.
            let one = Area {
                x0: x as u32,
                y0: y as u32,
                x1: x as u32 + 1,
                y1: y as u32 + 1,
            };
            area = Some(area.map_or(one, |area| area.with(one)));
            let mut add = |at: usize| {
                if hole[at] && !seen[at] {
                    seen[at] = true;
                    next.push(at);
                }
            };
            if x > 0 {
                add(at - 1);
            }
            if x + 1 < wide {
                add(at + 1);
            }
            if y > 0 {
                add(at - wide);
            }
            if y + 1 < tall {
                add(at + wide);
            }
        }
        out.extend(area);
    }
    // Letters of a word are one piece of work.
    let mut merged = true;
    while merged {
        merged = false;
        let mut kept: Vec<Area> = Vec::new();
        for area in out {
            let near = area.grown(reach, (w, h));
            match kept.iter_mut().find(|other| other.meets(near)) {
                Some(other) => {
                    *other = other.with(area);
                    merged = true;
                }
                None => kept.push(area),
            }
        }
        out = kept;
    }
    out
}

/// Where the squares of `side` begin to cover `from..to` of a line of `len`,
/// each showing `around` at both ends.
fn starts((from, to): (u32, u32), len: u32, side: u32, around: u32) -> Vec<u32> {
    let last = len.saturating_sub(side);
    if to - from + 2 * around <= side {
        // In the middle of one.
        let middle = (from + to) / 2;
        return vec![middle.saturating_sub(side / 2).min(last)];
    }
    let step = (side - 2 * around).max(1);
    let mut out = Vec::new();
    let mut at = from.saturating_sub(around).min(last);
    loop {
        out.push(at);
        if at >= last || at + side >= to + around {
            return out;
        }
        at = (at + step).min(last);
    }
}

impl Filler {
    /// What goes into the model for a square of the image.
    fn input(&self, square: &RgbImage, hole: &GrayImage) -> Vec<Vec<f32>> {
        let numbers = self.kind == Filling::Numbers;
        let most = if numbers { 255.0 } else { 1.0 };
        let image = (0..3)
            .flat_map(|c| square.pixels().map(move |p| f32::from(p.0[c]) / most))
            .collect();
        let mask = hole
            .pixels()
            .map(|p| match (numbers, p.0[0] > 0) {
                (true, true) => 1.0,
                (false, false) => 255.0,
                _ => 0.0,
            })
            .collect();
        vec![image, mask]
    }

    /// The square, filled.
    fn run(&self, square: &RgbImage, hole: &GrayImage) -> Result<RgbImage, VisionError> {
        let count = self.side * self.side;
        let out = self.session.run(self.input(square, hole))?;
        if out.len() != 3 * count {
            return Err(VisionError::Model("an odd output".to_owned()));
        }
        // Clamped: the cast drops the fraction alone.
        let byte = |v: f32| v.round().clamp(0.0, 255.0) as u8;
        // A model's side is small: the casts are exact.
        let side = self.side as u32;
        Ok(RgbImage::from_fn(side, side, |x, y| {
            let at = y as usize * self.side + x as usize;
            Rgb([0, 1, 2].map(|c| byte(out[c * count + at])))
        }))
    }

    /// Fills what `hole` covers of `work`, square after square.
    fn squares(
        &self,
        work: &mut RgbImage,
        hole: &mut GrayImage,
        mut done: impl FnMut(f32),
        cancel: &CancelToken,
    ) -> Result<(), VisionError> {
        let (w, h) = work.dimensions();
        let side = self.side as u32;
        // A share of a small side: the cast drops the fraction alone.
        let around = (side as f32 * AROUND) as u32;
        let covered = hole.enumerate_pixels().filter(|(_, _, p)| p.0[0] > 0);
        let whole = covered.fold(None::<Area>, |area, (x, y, _)| {
            let one = Area {
                x0: x,
                y0: y,
                x1: x + 1,
                y1: y + 1,
            };
            Some(area.map_or(one, |area| area.with(one)))
        });
        let Some(whole) = whole else { return Ok(()) };
        let columns = starts((whole.x0, whole.x1), w, side, around);
        let rows = starts((whole.y0, whole.y1), h, side, around);
        let count = (columns.len() * rows.len()) as f32;
        for (n, (x, y)) in rows
            .iter()
            .flat_map(|y| columns.iter().map(move |x| (*x, *y)))
            .enumerate()
        {
            cancelled(cancel)?;
            let piece = imageops::crop_imm(hole, x, y, side, side).to_image();
            if piece.pixels().any(|p| p.0[0] > 0) {
                let square = imageops::crop_imm(work, x, y, side, side).to_image();
                let filled = self.run(&square, &piece)?;
                for (dx, dy, p) in piece.enumerate_pixels() {
                    // What is at the edge of this one waits for the next,
                    // which sees what is past it; there is none past the
                    // image.
                    let inner = |d: u32, at: u32, len: u32, first: bool, last: bool| {
                        (d >= around || at == 0 || first)
                            && (d + around < side || at + side >= len || last)
                    };
                    let keep = inner(dx, x, w, columns[0] == x, columns.last() == Some(&x))
                        && inner(dy, y, h, rows[0] == y, rows.last() == Some(&y));
                    if p.0[0] > 0 && keep {
                        work.put_pixel(x + dx, y + dy, *filled.get_pixel(dx, dy));
                        hole.put_pixel(x + dx, y + dy, Luma([0]));
                    }
                }
            }
            done((n + 1) as f32 / count);
        }
        Ok(())
    }
}

/// `image` at `size`.
fn sized(image: &RgbImage, size: (u32, u32), filter: FilterType) -> RgbImage {
    if image.dimensions() == size {
        return image.clone();
    }
    imageops::resize(image, size.0, size.1, filter)
}

impl Inpainter for Filler {
    fn fill(
        &self,
        image: &Pixels,
        mask: &AlphaMask,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<Pixels, VisionError> {
        let (w, h) = image.size();
        if (mask.width(), mask.height()) != (w, h) {
            return Err(VisionError::Model("an odd mask".to_owned()));
        }
        cancelled(cancel)?;
        let wide = (w as usize, h as usize);
        // A little past what was marked: edges are soft.
        // A share of a side: the casts drop the fraction alone.
        let soft = (w.max(h) as f32 / 400.0).clamp(2.0, 12.0) as u32;
        let marked: Vec<bool> = mask.data().iter().map(|a| *a >= 64).collect();
        if !marked.contains(&true) {
            progress.report(1.0);
            return Ok(image.clone());
        }
        let far = distances(&marked, wide);
        let hole: Vec<bool> = far.iter().map(|d| *d <= soft * 3).collect();
        let kept: Vec<bool> = hole.iter().map(|hole| !hole).collect();
        let depth = distances(&kept, wide);
        let mut rgb = opaque(image);
        let side = self.side as u32;
        let list = areas(&hole, (w, h), side / 4);
        let share = 1.0 / list.len().max(1) as f32;
        for (n, area) in list.iter().enumerate() {
            let inside = |x: u32, y: u32| y as usize * wide.0 + x as usize;
            let deepest = (area.y0..area.y1)
                .flat_map(|y| (area.x0..area.x1).map(move |x| (x, y)))
                .map(|(x, y)| depth[inside(x, y)])
                .max()
                .unwrap_or(0);
            // From one edge of the widest place to the other, in px; a hole
            // that is all of the image has no edge.
            let across = (2 * deepest / 3).min(w.max(h)) as f32;
            let smaller = (across / (side as f32 * WIDEST)).max(1.0);
            let room = (side as f32 * smaller) as u32;
            let crop = area.grown(room / 2, (w, h));
            let (cw, ch) = crop.size();
            let to = |v: u32| ((v as f32 / smaller).round() as u32).max(1);
            let small = (to(cw), to(ch));
            let cut = imageops::crop_imm(&rgb, crop.x0, crop.y0, cw, ch).to_image();
            let shown = sized(&cut, small, FilterType::Triangle);
            // The model's square, and no less.
            let full = (small.0.max(side), small.1.max(side));
            let mut work = RgbImage::from_fn(full.0, full.1, |x, y| {
                *shown.get_pixel(mirrored(x, small.0), mirrored(y, small.1))
            });
            let at = |v: u32, from: u32, len: u32, of: u32| {
                // A pixel of the crop: within it.
                from + ((v as f32 + 0.5) * of as f32 / len as f32).min(of as f32 - 1.0) as u32
            };
            let mut open = GrayImage::from_fn(full.0, full.1, |x, y| {
                let (x, y) = (mirrored(x, small.0), mirrored(y, small.1));
                let (x, y) = (at(x, crop.x0, small.0, cw), at(y, crop.y0, small.1, ch));
                Luma([if hole[inside(x, y)] { 255 } else { 0 }])
            });
            if smaller > 1.0 {
                // A pixel made of several has a part of them.
                open = imageops::blur(&open, 0.8);
                open.pixels_mut().for_each(|p| {
                    p.0[0] = if p.0[0] > 0 { 255 } else { 0 };
                });
            }
            let told = |done: f32| progress.report((n as f32 + done) * share);
            self.squares(&mut work, &mut open, told, cancel)?;
            let filled = imageops::crop_imm(&work, 0, 0, small.0, small.1).to_image();
            let filled = sized(&filled, (cw, ch), FilterType::CatmullRom);
            for y in 0..ch {
                for x in 0..cw {
                    let (px, py) = (crop.x0 + x, crop.y0 + y);
                    let i = inside(px, py);
                    if !hole[i] {
                        continue;
                    }
                    // Whole where it was marked, softly outside.
                    let t = if marked[i] {
                        1.0
                    } else {
                        1.0 - far[i] as f32 / (3 * soft + 3) as f32
                    };
                    let (old, new) = (*rgb.get_pixel(px, py), *filled.get_pixel(x, y));
                    let mixed = [0, 1, 2].map(|c| {
                        let (a, b) = (f32::from(old.0[c]), f32::from(new.0[c]));
                        // Between two bytes: it is one.
                        (a + (b - a) * t).round() as u8
                    });
                    rgb.put_pixel(px, py, Rgb(mixed));
                }
            }
        }
        cancelled(cancel)?;
        let mut out = image.rgba().to_vec();
        for ((px, p), hole) in out.chunks_exact_mut(4).zip(rgb.pixels()).zip(&hole) {
            if *hole {
                px[..3].copy_from_slice(&p.0);
            }
        }
        progress.report(1.0);
        Pixels::new(w, h, out).ok_or(VisionError::TooLarge)
    }
}

/// `v` of a line of `len` that is seen again and again past its end.
fn mirrored(v: u32, len: u32) -> u32 {
    let period = 2 * len;
    let at = v % period;
    if at < len { at } else { period - 1 - at }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::NoProgress;
    use proptest::prelude::*;
    use std::sync::Mutex;

    use super::*;

    /// Fills with the mean of what is kept, and remembers the holes it saw.
    struct Mean {
        side: usize,
        kind: Filling,
        shares: Mutex<Vec<f32>>,
    }

    impl Session for Mean {
        fn run(&self, inputs: Vec<Vec<f32>>) -> Result<Vec<f32>, VisionError> {
            let count = self.side * self.side;
            assert_eq!((inputs[0].len(), inputs[1].len()), (3 * count, count));
            let hole = |i: usize| match self.kind {
                Filling::Numbers => inputs[1][i] > 0.5,
                Filling::Bytes => inputs[1][i] < 128.0,
            };
            let k = if self.kind == Filling::Numbers {
                255.0
            } else {
                1.0
            };
            let kept = (0..count).filter(|i| !hole(*i)).count();
            self.shares
                .lock()
                .unwrap()
                .push(1.0 - kept as f32 / count as f32);
            let mut out = Vec::with_capacity(3 * count);
            for c in 0..3 {
                let sum: f32 = (0..count)
                    .filter(|i| !hole(*i))
                    .map(|i| inputs[0][c * count + i])
                    .sum();
                let mean = sum / kept.max(1) as f32 * k;
                out.extend((0..count).map(|i| {
                    if hole(i) {
                        mean
                    } else {
                        inputs[0][c * count + i] * k
                    }
                }));
            }
            Ok(out)
        }
    }

    fn filler(side: usize, kind: Filling) -> (Filler, Arc<Mean>) {
        let session = Arc::new(Mean {
            side,
            kind,
            shares: Mutex::default(),
        });
        let filler = Filler {
            session: Arc::clone(&session) as Arc<dyn Session>,
            side,
            kind,
        };
        (filler, session)
    }

    /// Grey with a red box to erase.
    fn scene(w: u32, h: u32, (x0, y0, x1, y1): (u32, u32, u32, u32)) -> (Pixels, AlphaMask) {
        let on = |i: u32| {
            let (x, y) = (i % w, i / w);
            x >= x0 && x < x1 && y >= y0 && y < y1
        };
        let bytes = (0..w * h).flat_map(|i| {
            if on(i) {
                [255, 0, 0, 200]
            } else {
                [120, 124, 128, 255]
            }
        });
        let mask = (0..w * h).map(|i| if on(i) { 255 } else { 0 }).collect();
        (
            Pixels::new(w, h, bytes.collect()).unwrap(),
            AlphaMask::new(w, h, mask).unwrap(),
        )
    }

    // S86-02: what is marked goes, the opacity stays, and away from it every
    // pixel is the same; with both kinds of models, whatever the sizes.
    #[test]
    fn s86_02_what_is_marked_goes() {
        for kind in [Filling::Bytes, Filling::Numbers] {
            for (w, h, hole) in [
                (40, 30, (10, 10, 18, 16)),
                (300, 200, (100, 20, 180, 190)),
                (500, 60, (3, 20, 497, 30)),
                (90, 400, (0, 0, 90, 60)),
            ] {
                let (image, mask) = scene(w, h, hole);
                let (filler, _) = filler(64, kind);
                let out = filler
                    .fill(&image, &mask, &NoProgress, &CancelToken::new())
                    .unwrap();
                assert_eq!(out.size(), (w, h));
                for (i, (a, b)) in out.rgba().chunks(4).zip(image.rgba().chunks(4)).enumerate() {
                    let (x, y) = (i as u32 % w, i as u32 / w);
                    assert_eq!(a[3], b[3]);
                    if mask.data()[i] > 0 {
                        assert!(a[0] < 170 && a[1] > 70, "{kind:?} {w}×{h} {x},{y}: {a:?}");
                    }
                    let far =
                        x + 14 < hole.0 || x > hole.2 + 14 || y + 14 < hole.1 || y > hole.3 + 14;
                    if far {
                        assert_eq!(a, b, "{x},{y}");
                    }
                }
            }
        }
    }

    // S86-02: the model always sees something around a wide hole, and a thin
    // one is seen at its own size.
    #[test]
    fn s86_02_a_wide_hole_is_seen_smaller() {
        let (image, mask) = scene(600, 600, (150, 150, 450, 450));
        let (filler, model) = filler(64, Filling::Numbers);
        filler
            .fill(&image, &mask, &NoProgress, &CancelToken::new())
            .unwrap();
        let shares = model.shares.lock().unwrap().clone();
        assert!(shares.iter().all(|s| *s < 0.7), "{shares:?}");
        let (image, mask) = scene(600, 100, (20, 48, 580, 52));
        let (filler, model) = self::filler(64, Filling::Numbers);
        filler
            .fill(&image, &mask, &NoProgress, &CancelToken::new())
            .unwrap();
        assert!(model.shares.lock().unwrap().len() > 8, "at its size");
    }

    // S86-01: nothing marked changes nothing; another mask, an odd output or
    // a cancelled one are told.
    #[test]
    fn s86_01_the_contract() {
        let (image, mask) = scene(20, 20, (5, 5, 9, 9));
        let (ok, model) = filler(32, Filling::Numbers);
        let none = AlphaMask::new(20, 20, vec![0; 400]).unwrap();
        let fill =
            |f: &Filler, mask: &AlphaMask| f.fill(&image, mask, &NoProgress, &CancelToken::new());
        assert_eq!(fill(&ok, &none), Ok(image.clone()));
        assert!(model.shares.lock().unwrap().is_empty());
        let other = AlphaMask::new(10, 40, vec![0; 400]).unwrap();
        assert!(matches!(fill(&ok, &other), Err(VisionError::Model(_))));
        struct Odd;
        impl Session for Odd {
            fn run(&self, _: Vec<Vec<f32>>) -> Result<Vec<f32>, VisionError> {
                Ok(vec![1.0])
            }
        }
        let odd = Filler {
            session: Arc::new(Odd),
            side: 32,
            kind: Filling::Bytes,
        };
        assert!(matches!(fill(&odd, &mask), Err(VisionError::Model(_))));
        let token = CancelToken::new();
        token.cancel();
        let out = ok.fill(&image, &mask, &NoProgress, &token);
        assert_eq!(out, Err(VisionError::Cancelled));
    }

    proptest! {
        // S86-02: any hole in any image, and nothing red is left of it.
        #[test]
        fn s86_02_any_hole(w in 1_u32..120, h in 1_u32..120, a in (0.0..1.0_f32, 0.0..1.0_f32), b in (0.0..1.0_f32, 0.0..1.0_f32)) {
            let at = |v: f32, len: u32| ((v * len as f32) as u32).min(len - 1);
            let (x0, x1) = (at(a.0.min(b.0), w), at(a.0.max(b.0), w) + 1);
            let (y0, y1) = (at(a.1.min(b.1), h), at(a.1.max(b.1), h) + 1);
            let (image, mask) = scene(w, h, (x0, y0, x1, y1));
            let (filler, _) = filler(32, Filling::Bytes);
            let out = filler.fill(&image, &mask, &NoProgress, &CancelToken::new()).unwrap();
            prop_assert_eq!(out.size(), (w, h));
            // With room around, what was marked is gone.
            if x0 > 16 && y0 > 16 && x1 + 16 < w && y1 + 16 < h {
                let left = out.rgba().chunks(4).zip(mask.data()).filter(|(p, m)| **m > 0 && p[0] > 200).count();
                prop_assert_eq!(left, 0);
            }
        }
    }
}
