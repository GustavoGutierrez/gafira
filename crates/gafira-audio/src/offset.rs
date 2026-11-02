//! Flick offsets to samples (ARQ-129, FV-10 d).

use gafira_time::{FLICKS_PER_SECOND, Flicks, SampleRate};

/// `round(offset × fs / 705 600 000)`, half away from 0, in integer
/// arithmetic: nothing overflows and no `f64` takes part.
#[must_use]
pub fn flicks_to_samples(offset: Flicks, rate: SampleRate) -> i64 {
    let scaled = i128::from(offset.ticks()) * i128::from(rate.hz());
    let per_second = i128::from(FLICKS_PER_SECOND);
    let mut samples = scaled / per_second;
    if (scaled % per_second).abs() * 2 >= per_second {
        samples += scaled.signum();
    }
    // A valid rate is at most 705 600 000 Hz, so |samples| <= |flicks|.
    i64::try_from(samples).unwrap_or(if samples < 0 { i64::MIN } else { i64::MAX })
}
