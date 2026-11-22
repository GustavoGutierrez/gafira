//! A shape out of focus (spec 089): its coverage blurred, in one colour.

use gafira_color::Color;
use kurbo::{Affine, BezPath, PathEl, Point, Shape};

use crate::effects::blur_plane;
use crate::transfer::Transfer;
use crate::{ImageBuf, PixelBuf};

/// The longest side of the picture of a shape.
const LONGEST: f64 = 2048.0;

/// The samples of a pixel, across and down.
const SAMPLES: usize = 4;

/// `path` in `color`, blurred by `sigma`, at `density` pixels for each unit
/// or fewer, as nothing fine is left of it; and where the first pixel is and
/// the size of each, in the units of the path. None when there is nothing.
#[must_use]
pub fn soft_shape(
    path: &BezPath,
    color: Color,
    sigma: f64,
    density: f64,
) -> Option<(ImageBuf, Affine)> {
    let bounds = path.bounding_box();
    let sigma = sigma.max(0.0);
    let far = 3.0 * sigma + 1.0;
    let (w, h) = (bounds.width() + 2.0 * far, bounds.height() + 2.0 * far);
    let sound = [bounds.x0, bounds.y0, w, h, density]
        .iter()
        .all(|v| v.is_finite());
    if !sound || bounds.width() <= 0.0 || bounds.height() <= 0.0 || density <= 0.0 {
        return None;
    }
    let enough = if sigma > 0.0 {
        6.0 / sigma
    } else {
        f64::INFINITY
    };
    let k = density.min(enough).min(LONGEST / w.max(h));
    // At most LONGEST: the casts are exact.
    let side = |v: f64| ((v * k).ceil() as u32).max(1);
    let (pw, ph) = (side(w), side(h));
    let place = Affine::scale(k) * Affine::translate((far - bounds.x0, far - bounds.y0));
    let cover = coverage(&(place * path.clone()), (pw as usize, ph as usize));
    let soft = blur_plane(&cover, pw as usize, ph as usize, 2.0 * sigma * k);
    let linear = color.to_linear_srgb();
    let [r, g, b] = Transfer::new()
        .encode([linear.r, linear.g, linear.b].map(|c| f64::from(c).clamp(0.0, 1.0)))
        .map(f32::from);
    let alpha = color.alpha().clamp(0.0, 1.0);
    let mut data = Vec::with_capacity(soft.len() * 4);
    for v in soft {
        let a = (v * alpha).clamp(0.0, 1.0);
        let top = (a * 255.0).round();
        // Between 0 and 255: the cast is exact.
        data.extend([r, g, b, 255.0].map(|c| (c * a).round().min(top) as u8));
    }
    let image = ImageBuf::new(pw, ph, PixelBuf::Rgba8(data)).ok()?;
    Some((image, place.inverse()))
}

/// How much of each pixel is inside `path` (non-zero winding).
fn coverage(path: &BezPath, (w, h): (usize, usize)) -> Vec<f32> {
    // Where each line crosses each row of samples, and which way.
    let mut rows: Vec<Vec<(f64, i32)>> = vec![Vec::new(); h * SAMPLES];
    let k = SAMPLES as f64;
    let mut edge = |a: Point, b: Point| {
        if a.y == b.y {
            return;
        }
        let (top, low, way) = if a.y < b.y { (a, b, 1) } else { (b, a, -1) };
        // The rows whose middle is between both ends.
        let first = (top.y * k - 0.5).ceil().max(0.0);
        let last = (low.y * k - 0.5).ceil().min((h * SAMPLES) as f64);
        let mut row = first;
        while row < last {
            let y = (row + 0.5) / k;
            let x = top.x + (y - top.y) / (low.y - top.y) * (low.x - top.x);
            // A row of the picture: the cast is exact.
            rows[row as usize].push((x, way));
            row += 1.0;
        }
    };
    let (mut start, mut at) = (Point::ZERO, Point::ZERO);
    kurbo::flatten(path.iter(), 0.1, |el| match el {
        PathEl::MoveTo(p) => {
            edge(at, start);
            (start, at) = (p, p);
        }
        PathEl::LineTo(p) => {
            edge(at, p);
            at = p;
        }
        PathEl::ClosePath => {
            edge(at, start);
            at = start;
        }
        // Flattened: there are no curves.
        PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
    });
    edge(at, start);
    let mut out = vec![0.0_f32; w * h];
    let share = 1.0 / SAMPLES as f32;
    for (row, crossings) in rows.iter_mut().enumerate() {
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let line = &mut out[row / SAMPLES * w..][..w];
        let mut winding = 0;
        for pair in crossings.windows(2) {
            winding += pair[0].1;
            if winding == 0 {
                continue;
            }
            let (from, to) = (
                pair[0].0.clamp(0.0, w as f64),
                pair[1].0.clamp(0.0, w as f64),
            );
            if to <= from {
                continue;
            }
            // Inside the row: the casts drop the fraction alone.
            let (first, last) = (from as usize, (to.ceil() as usize).min(w));
            for (x, v) in line.iter_mut().enumerate().take(last).skip(first) {
                let part = to.min(x as f64 + 1.0) - from.max(x as f64);
                *v += part.max(0.0) as f32 * share;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_color::Rgba;
    use proptest::prelude::*;

    fn total(path: &BezPath, sigma: f64, density: f64) -> (f64, Affine, ImageBuf) {
        let red = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
        let (image, at) = soft_shape(path, red, sigma, density).expect("a shape");
        let PixelBuf::Rgba8(data) = image.data() else {
            panic!("8 bits")
        };
        assert!(data.chunks(4).all(|p| p[0] == p[3] && p[1] == 0));
        let sum: f64 = data.chunks(4).map(|p| f64::from(p[3]) / 255.0).sum();
        let pixel = at.as_coeffs()[0];
        (sum * pixel * pixel, at, image)
    }

    // S89-02: what there is of it is the same however blurred it is, and it
    // is where it was.
    #[test]
    fn s89_02_a_soft_shape() {
        let square = kurbo::Rect::new(10.0, 20.0, 50.0, 40.0).to_path(0.1);
        for (sigma, density) in [(0.0, 1.0), (0.0, 3.0), (2.0, 2.0), (12.0, 2.0)] {
            let (area, at, image) = total(&square, sigma, density);
            assert!((area - 800.0).abs() < 12.0, "{sigma} {density} {area}");
            let first = at * Point::ZERO;
            assert!((first.x - (10.0 - 3.0 * sigma - 1.0)).abs() < 1e-9);
            let last = at * Point::new(f64::from(image.width()), f64::from(image.height()));
            assert!(last.x >= 50.0 + 3.0 * sigma && last.y >= 40.0 + 3.0 * sigma);
        }
        // A hole stays, and a piece over another counts once.
        let mut ring = kurbo::Rect::new(0.0, 0.0, 40.0, 40.0).to_path(0.1);
        ring.extend(
            kurbo::Rect::new(10.0, 10.0, 30.0, 30.0)
                .to_path(0.1)
                .reverse_subpaths(),
        );
        assert!((total(&ring, 0.0, 1.0).0 - 1200.0).abs() < 8.0);
        let mut twice = square.clone();
        twice.extend(kurbo::Rect::new(20.0, 20.0, 60.0, 40.0).to_path(0.1));
        assert!((total(&twice, 0.0, 1.0).0 - 1000.0).abs() < 8.0);
        let red = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
        assert!(soft_shape(&BezPath::new(), red, 2.0, 1.0).is_none());
        assert!(soft_shape(&square, red, 2.0, 0.0).is_none());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(24))]
        // S89-02: any shape, any blur: no failure, and not too large.
        #[test]
        fn s89_02_any_shape(points in prop::collection::vec((-5e3..5e3_f64, -5e3..5e3_f64), 0..12), sigma in 0.0..900.0_f64, density in 0.0..40.0_f64) {
            let mut path = BezPath::new();
            for (i, p) in points.iter().enumerate() {
                if i % 5 == 0 { path.move_to(*p); } else { path.line_to(*p); }
            }
            let ink = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 0.5));
            if let Some((image, _)) = soft_shape(&path, ink, sigma, density) {
                prop_assert!(image.width() <= 2049 && image.height() <= 2049);
            }
        }
    }
}
