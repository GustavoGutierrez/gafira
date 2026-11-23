//! The axis of the values (S83-04) and how its numbers are written.

/// Round numbers from at or below `min` to at or over `max`, about `most`
/// steps of one, two or five times a power of ten.
#[must_use]
pub fn ticks(min: f64, max: f64, most: usize) -> Vec<f64> {
    if !(min.is_finite() && max.is_finite()) || max < min {
        return vec![0.0, 1.0];
    }
    let size = min.abs().max(max.abs());
    let length = max - min;
    // A lone value, or as good as one.
    if length <= size * 1e-12 {
        return vec![min, min + (size * 0.1).max(1.0)];
    }
    // Far below 2^53: the cast is exact.
    let raw = length / most.max(1) as f64;
    // Finite and within the powers of an f64: the cast is exact.
    let mut power = raw.log10().floor() as i32;
    let part = raw / 10_f64.powi(power);
    let mut step = 10;
    for nice in [1, 2, 5] {
        if part <= f64::from(nice) * (1.0 + 1e-9) {
            step = nice;
            break;
        }
    }
    if step == 10 {
        step = 1;
        power += 1;
    }
    let step = f64::from(step);
    // A power below one divides, so tenths are written as tenths.
    let at = |i: f64| {
        if power < 0 {
            i * step / 10_f64.powi(-power)
        } else {
            i * step * 10_f64.powi(power)
        }
    };
    let unit = at(1.0);
    let slack = 1e-9;
    let first = (min / unit + slack).floor();
    let last = (max / unit - slack).ceil().max(first + 1.0);
    // Few steps: the cast is exact.
    let count = (last - first).min(1000.0) as u32;
    (0..=count).map(|i| at(first + f64::from(i))).collect()
}

/// `v` with at most `places` decimals, the last zeros left out.
pub(crate) fn number(v: f64, places: usize, comma: bool) -> String {
    let mut out = format!("{v:.places$}");
    if out.contains('.') {
        let kept = out.trim_end_matches('0').trim_end_matches('.').len();
        out.truncate(kept);
    }
    if out == "-0" {
        out = "0".to_owned();
    }
    if comma { out.replace('.', ",") } else { out }
}

/// `v` as an axis writes it: short from ten thousand on, the most of the
/// axis being `most`.
pub(crate) fn short(v: f64, most: f64, comma: bool) -> String {
    let (unit, mark) = if most >= 1e6 {
        (1e6, " M")
    } else if most >= 1e4 {
        (1e3, " k")
    } else {
        (1.0, "")
    };
    let text = number(v / unit, 6, comma);
    if v == 0.0 {
        text
    } else {
        format!("{text}{mark}")
    }
}
