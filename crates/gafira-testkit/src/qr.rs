//! A reader for the tests of the QR codes (spec 080, FD-20 a).

/// Dark is below the middle grey.
const MIDDLE: u8 = 128;

/// The text of a code of `size` modules a side, `grey(x, y)` the light at a
/// point in modules from the corner of its first one. `None` when an eye
/// would not be found or the code does not decode.
///
/// The caller knows where the code lies. What it checks is what a reader
/// needs: every eye seen as the runs 1:1:3:1:1 across and down its middle,
/// with the half module of slack readers allow, and modules that decode.
pub fn read(size: usize, grey: impl Fn(f64, f64) -> u8) -> Option<String> {
    // At most 177: the cast is exact.
    let far = size as f64 - 7.0;
    for (x, y) in [(0.0, 0.0), (far, 0.0), (0.0, far)] {
        let across = |t: f64| grey(x + t, y + 3.5) < MIDDLE;
        let down = |t: f64| grey(x + 3.5, y + t) < MIDDLE;
        if !is_eye(across) || !is_eye(down) {
            return None;
        }
    }
    let modules =
        rqrr::SimpleGrid::from_func(size, |x, y| grey(x as f64 + 0.5, y as f64 + 0.5) < MIDDLE);
    let (_, text) = rqrr::Grid::new(modules).decode().ok()?;
    Some(text)
}

/// Whether the runs from the middle of an eye outwards are 1:1:3:1:1.
fn is_eye(dark: impl Fn(f64) -> bool) -> bool {
    const STEPS: i32 = 16;
    let at = |step: i32| dark(f64::from(step) / f64::from(STEPS) + 1.0 / 32.0);
    let runs = |way: i32| {
        let mut lengths = [0.0; 3];
        let mut step = 7 * STEPS / 2 - i32::from(way < 0);
        for (run, length) in lengths.iter_mut().enumerate() {
            // Dark, light, dark; the last one ends in the light around.
            while at(step) == (run != 1) && (-STEPS..8 * STEPS).contains(&step) {
                *length += 1.0 / f64::from(STEPS);
                step += way;
            }
        }
        lengths
    };
    let (left, right) = (runs(-1), runs(1));
    let middle = left[0] + right[0];
    (middle - 3.0).abs() < 1.5
        && [left[1], left[2], right[1], right[2]]
            .iter()
            .all(|run: &f64| (run - 1.0).abs() < 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_eye_is_1_1_3_1_1() {
        let eye = |t: f64| {
            (0.0..7.0).contains(&t) && !(1.0..2.0).contains(&t) && !(5.0..6.0).contains(&t)
        };
        assert!(is_eye(eye));
        // A gap across the middle is not one.
        assert!(!is_eye(|t| eye(t) && !(3.2..3.8).contains(&t)));
        assert!(!is_eye(|t| (0.0..7.0).contains(&t)));
        assert!(!is_eye(|_| false));
    }
}
